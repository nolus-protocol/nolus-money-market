use std::result::Result as StdResult;

use finance::{
    coin::{Coin, WithCoin},
    currency::{Currency, Group},
    error::Error as FinanceError,
};
use sdk::cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, Env, QuerierWrapper};

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

pub fn received_any<G, V>(cw_amount: Vec<CwCoin>, cmd: V) -> StdResult<V::Output, V::Error>
where
    V: WithCoin,
    G: Group,
    FinanceError: Into<V::Error>,
    Error: Into<V::Error>,
{
    received_one_impl(cw_amount, Error::NoFundsAny, Error::UnexpectedFundsAny)
        .map_err(Into::into)
        .and_then(|coin| from_cosmwasm_any_impl::<G, _>(coin, cmd))
}

pub struct BankView<'a> {
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> BankView<'a> {
    pub fn my_account(env: &'a Env, querier: &'a QuerierWrapper) -> Self {
        Self {
            addr: &env.contract.address,
            querier,
        }
    }
}

impl<'a> BankAccountView for BankView<'a> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        let coin = self.querier.query_balance(self.addr, C::BANK_SYMBOL)?;
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

pub fn my_account<'a>(env: &'a Env, querier: &'a QuerierWrapper) -> BankStub<BankView<'a>> {
    BankStub::new(BankView::my_account(env, querier))
}

#[cfg(feature = "testing")]
pub fn balance<'a, C>(addr: &'a Addr, querier: &'a QuerierWrapper<'a>) -> Result<Coin<C>>
where
    C: Currency,
{
    BankView { addr, querier }.balance()
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
