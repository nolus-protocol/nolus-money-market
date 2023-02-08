use finance::{
    coin::{Coin, WithCoin, WithCoinResult},
    currency::{Currency, Group},
};
use sdk::cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, QuerierWrapper};

use crate::{
    batch::Batch,
    coin_legacy::{from_cosmwasm_any_impl, from_cosmwasm_impl, to_cosmwasm_impl},
    error::{Error, Result},
};

pub trait BankAccountView {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency;
}

pub trait BankAccount
where
    Self: BankAccountView + Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>, to: &Addr)
    where
        C: Currency;
}

pub trait FixedAddressSender
where
    Self: Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: Currency;
}

/// Ensure a single coin of the specified currency is received by a contract and return it
pub fn received_one<C>(cw_amount: Vec<CwCoin>) -> Result<Coin<C>>
where
    C: Currency,
{
    received_one_impl(
        cw_amount,
        Error::no_funds::<C>,
        Error::unexpected_funds::<C>,
    )
    .and_then(from_cosmwasm_impl)
}

/// Run a command on the first coin of the specified group
pub fn may_received<G, V>(cw_amount: Vec<CwCoin>, mut cmd: V) -> Option<WithCoinResult<V>>
where
    V: WithCoin,
    G: Group,
{
    let mut may_res = None;
    for coin in cw_amount {
        cmd = match from_cosmwasm_any_impl::<G, _>(coin, cmd) {
            Ok(res) => {
                may_res = Some(res);
                break;
            }
            Err(cmd) => cmd,
        }
    }
    may_res
}

pub struct BankView<'a> {
    account: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> BankView<'a> {
    fn account(account: &'a Addr, querier: &'a QuerierWrapper<'a>) -> Self {
        Self { account, querier }
    }
}

impl<'a> BankAccountView for BankView<'a> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        let coin = self.querier.query_balance(self.account, C::BANK_SYMBOL)?;
        from_cosmwasm_impl(coin)
    }
}

pub struct BankStub<View>
where
    View: BankAccountView,
{
    view: View,
    batch: Batch,
}

impl<View> BankStub<View>
where
    View: BankAccountView,
{
    pub fn new(view: View) -> Self {
        Self {
            view,
            batch: Batch::default(),
        }
    }
}

pub fn account<'a>(account: &'a Addr, querier: &'a QuerierWrapper<'a>) -> BankStub<BankView<'a>> {
    BankStub::new(BankView::account(account, querier))
}

pub fn balance<'a, C>(account: &'a Addr, querier: &'a QuerierWrapper<'a>) -> Result<Coin<C>>
where
    C: Currency,
{
    BankView { account, querier }.balance()
}

impl<View> BankAccountView for BankStub<View>
where
    View: BankAccountView,
{
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        self.view.balance()
    }
}

impl<View> BankAccount for BankStub<View>
where
    Self: BankAccountView + Into<Batch>,
    View: BankAccountView,
{
    fn send<C>(&mut self, amount: Coin<C>, to: &Addr)
    where
        C: Currency,
    {
        debug_assert!(!amount.is_zero());
        self.batch.schedule_execute_no_reply(BankMsg::Send {
            to_address: to.into(),
            amount: vec![to_cosmwasm_impl(amount)],
        });
    }
}

impl<View> From<BankStub<View>> for Batch
where
    View: BankAccountView,
{
    fn from(stub: BankStub<View>) -> Self {
        stub.batch
    }
}

fn received_one_impl<NoFundsErr, UnexpFundsErr>(
    cw_amount: Vec<CwCoin>,
    no_funds_err: NoFundsErr,
    unexp_funds_err: UnexpFundsErr,
) -> Result<CwCoin>
where
    NoFundsErr: FnOnce() -> Error,
    UnexpFundsErr: FnOnce() -> Error,
{
    match cw_amount.len() {
        0 => Err(no_funds_err()),
        1 => {
            let first = cw_amount
                .into_iter()
                .next()
                .expect("there is at least a coin");
            Ok(first)
        }
        _ => Err(unexp_funds_err()),
    }
}

pub struct LazySenderStub {
    receiver: Addr,
    amounts: Vec<CwCoin>,
}

impl LazySenderStub {
    pub fn new(receiver: Addr) -> Self {
        Self {
            receiver,
            amounts: Vec::new(),
        }
    }
}

impl FixedAddressSender for LazySenderStub
where
    Self: Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: Currency,
    {
        debug_assert!(!amount.is_zero());

        if amount.is_zero() {
            return;
        }

        self.amounts.push(to_cosmwasm_impl(amount));
    }
}

impl From<LazySenderStub> for Batch {
    fn from(stub: LazySenderStub) -> Self {
        let mut batch = Batch::default();

        if !stub.amounts.is_empty() {
            batch.schedule_execute_no_reply(BankMsg::Send {
                to_address: stub.receiver.to_string(),
                amount: stub.amounts,
            });
        }

        batch
    }
}

#[cfg(test)]
mod test {
    use finance::{
        coin::{Amount, Coin},
        currency::{Currency, SymbolStatic},
        test::{
            coin::Expect,
            currency::{Dai, TestCurrencies, Usdc},
        },
    };

    use crate::coin_legacy;

    use super::may_received;
    type TheCurrency = Usdc;
    type ExtraCurrency = Dai;
    const AMOUNT: Amount = 42;

    #[test]
    fn may_received_no_input() {
        assert_eq!(
            None,
            may_received::<TestCurrencies, _>(vec![], Expect(Coin::<TheCurrency>::from(AMOUNT)))
        );
    }

    #[test]
    fn may_received_not_in_group() {
        let coin = Coin::<ExtraCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm(coin);

        #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
        struct MyNiceCurrency {}
        impl Currency for MyNiceCurrency {
            const BANK_SYMBOL: SymbolStatic = "wdd";
            const DEX_SYMBOL: SymbolStatic = "dex3rdf";
            const TICKER: SymbolStatic = "ticedc";
        }
        let in_coin_2 = coin_legacy::to_cosmwasm(Coin::<MyNiceCurrency>::new(AMOUNT));

        assert_eq!(
            None,
            may_received::<TestCurrencies, _>(vec![in_coin_1, in_coin_2], Expect(coin))
        );
    }

    #[test]
    fn may_received_in_group() {
        let coin = Coin::<TheCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm(coin);
        assert_eq!(
            Some(Ok(true)),
            may_received::<TestCurrencies, _>(vec![in_coin_1], Expect(coin))
        );
    }

    #[test]
    fn may_received_in_group_others_arround() {
        let in_coin_1 = coin_legacy::to_cosmwasm(Coin::<ExtraCurrency>::new(AMOUNT + AMOUNT));

        let coin_2 = Coin::<TheCurrency>::new(AMOUNT);
        let in_coin_2 = coin_legacy::to_cosmwasm(coin_2);

        let coin_3 = Coin::<TheCurrency>::new(AMOUNT + AMOUNT);
        let in_coin_3 = coin_legacy::to_cosmwasm(coin_3);
        assert_eq!(
            Some(Ok(true)),
            may_received::<TestCurrencies, _>(
                vec![in_coin_1.clone(), in_coin_2.clone(), in_coin_3.clone()],
                Expect(coin_2)
            )
        );
        assert_eq!(
            Some(Ok(true)),
            may_received::<TestCurrencies, _>(
                vec![in_coin_1, in_coin_3, in_coin_2],
                Expect(coin_3)
            )
        );
    }
}
