use std::{marker::PhantomData, result::Result as StdResult};

use currency::{AnyVisitor, AnyVisitorResult, Currency, Definition, Group, MemberOf};
use finance::coin::{Coin, WithCoin, WithCoinResult};
use sdk::cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, QuerierWrapper};

use crate::{
    batch::Batch,
    coin_legacy::{self, from_cosmwasm_any, maybe_from_cosmwasm_any, to_cosmwasm_impl},
    error::Error,
    result::Result,
};

pub type BalancesResult<G, Cmd> = StdResult<Option<WithCoinResult<G, Cmd>>, Error>;

pub trait BankAccountView {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency;

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G, VisitorG = G> + Clone,
        Cmd::Output: Aggregate;
}

pub trait BankAccount
where
    Self: BankAccountView + Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>, to: Addr)
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
    C: Currency + Definition,
{
    received_one_impl(
        cw_amount,
        Error::no_funds::<C>,
        Error::unexpected_funds::<C>,
    )
    .and_then(coin_legacy::from_cosmwasm::<C>)
}

/// Run a command on the first coin of the specified group
pub fn may_received<VisitedG, V>(
    cw_amount: &Vec<CwCoin>,
    mut cmd: V,
) -> Option<WithCoinResult<VisitedG, V>>
where
    VisitedG: Group,
    V: WithCoin<VisitedG, VisitorG = VisitedG>,
{
    let mut may_res = None;
    for coin in cw_amount {
        cmd = match from_cosmwasm_any(coin, cmd) {
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
    querier: QuerierWrapper<'a>,
}

impl<'a> BankView<'a> {
    fn account(account: &'a Addr, querier: QuerierWrapper<'a>) -> Self {
        Self { account, querier }
    }
}

impl<'a> BankAccountView for BankView<'a> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        struct QueryBankBalance<'a, C> {
            c: PhantomData<C>,
            account: &'a Addr,
            querier: QuerierWrapper<'a>,
        }
        impl<'a, C> AnyVisitor<C::Group> for QueryBankBalance<'a, C>
        where
            C: Currency,
        {
            type VisitorG = C::Group;

            type Output = Coin<C>;

            type Error = Error;

            fn on<CC>(self) -> AnyVisitorResult<C::Group, Self>
            where
                CC: Currency + Definition + MemberOf<Self::VisitorG>,
            {
                debug_assert!(currency::equal::<C, CC>());
                self.querier
                    .query_balance(self.account, CC::BANK_SYMBOL)
                    .map_err(Error::CosmWasmQueryBalance)
                    .and_then(coin_legacy::from_cosmwasm_currency_not_definition::<CC, C>)
            }
        }
        currency::dto::<C, C::Group>().into_currency_type(QueryBankBalance {
            c: PhantomData,
            account: self.account,
            querier: self.querier,
        })
    }

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G, VisitorG = G> + Clone,
        Cmd::Output: Aggregate,
    {
        self.querier
            .query_all_balances(self.account)
            .map_err(Error::CosmWasmQueryAllBalances)
            .map(|cw_coins| {
                cw_coins
                    .into_iter()
                    .filter_map(|cw_coin| maybe_from_cosmwasm_any::<G, _>(cw_coin, cmd.clone()))
                    .reduce_results(Aggregate::aggregate)
            })
            .map_err(Into::into)
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
    fn new(view: View) -> Self {
        Self {
            view,
            batch: Batch::default(),
        }
    }

    #[cfg(feature = "testing")]
    pub fn with_view(view: View) -> Self {
        Self::new(view)
    }
}

pub fn account<'a>(account: &'a Addr, querier: QuerierWrapper<'a>) -> BankStub<BankView<'a>> {
    BankStub::new(BankView::account(account, querier))
}

pub fn balance<'a, C>(account: &'a Addr, querier: QuerierWrapper<'a>) -> Result<Coin<C>>
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

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G, VisitorG = G> + Clone,
        Cmd::Output: Aggregate,
    {
        self.view.balances::<G, Cmd>(cmd)
    }
}

impl<View> BankAccount for BankStub<View>
where
    Self: BankAccountView + Into<Batch>,
    View: BankAccountView,
{
    fn send<C>(&mut self, amount: Coin<C>, to: Addr)
    where
        C: Currency,
    {
        debug_assert!(!amount.is_zero());
        bank_send_impl(&mut self.batch, to, &[amount])
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

fn bank_send_impl<C>(batch: &mut Batch, to: Addr, amount: &[Coin<C>])
where
    C: Currency,
{
    bank_send_cosmwasm(
        batch,
        to,
        amount
            .iter()
            .map(|coin| to_cosmwasm_impl(coin.to_owned()))
            .collect(),
    )
}

fn bank_send_cosmwasm(batch: &mut Batch, to: Addr, amount: Vec<CwCoin>) {
    batch.schedule_execute_no_reply(BankMsg::Send {
        amount,
        to_address: to.into(),
    });
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
        if !amount.is_zero() {
            self.amounts.push(to_cosmwasm_impl(amount));
        }
    }
}

impl From<LazySenderStub> for Batch {
    fn from(stub: LazySenderStub) -> Self {
        let mut batch = Batch::default();

        if !stub.amounts.is_empty() {
            bank_send_cosmwasm(&mut batch, stub.receiver, stub.amounts);
        }

        batch
    }
}

pub trait Aggregate {
    fn aggregate(self, other: Self) -> Self
    where
        Self: Sized;
}

impl Aggregate for () {
    fn aggregate(self, _: Self) -> Self {}
}

impl Aggregate for Batch {
    fn aggregate(self, other: Self) -> Self {
        self.merge(other)
    }
}

impl<T> Aggregate for Vec<T> {
    fn aggregate(mut self, mut other: Self) -> Self {
        self.append(&mut other);

        self
    }
}

/// Temporary replacement for functionality similar to
/// [`Iterator::try_reduce`] until the feature is stabilized.
trait ReduceResults
where
    Self: Iterator<Item = StdResult<Self::InnerItem, Self::Error>>,
{
    type InnerItem;
    type Error;

    fn reduce_results<F>(&mut self, f: F) -> Option<StdResult<Self::InnerItem, Self::Error>>
    where
        F: FnMut(Self::InnerItem, Self::InnerItem) -> Self::InnerItem;
}

impl<I, T, E> ReduceResults for I
where
    I: Iterator<Item = StdResult<T, E>>,
{
    type InnerItem = T;
    type Error = E;

    fn reduce_results<F>(&mut self, mut f: F) -> Option<StdResult<T, E>>
    where
        F: FnMut(T, T) -> T,
    {
        self.next().map(|first: StdResult<T, E>| {
            first.and_then(|first: T| {
                self.try_fold(first, |acc: T, element: StdResult<T, E>| {
                    element.map(|element: T| f(acc, element))
                })
            })
        })
    }
}

/// Send a single coin to a recepient
#[cfg(feature = "testing")]
pub fn bank_send<C>(to: Addr, amount: Coin<C>) -> Batch
where
    C: Currency,
{
    let mut batch = Batch::default();
    bank_send_impl(&mut batch, to, &[amount]);
    batch
}

#[cfg(test)]
mod test {

    use currency::{
        test::{SubGroup, SubGroupTestC1, SuperGroupTestC1},
        Currency, CurrencyDTO, Definition, Group, MemberOf,
    };
    use finance::{
        coin::{Amount, Coin, WithCoin, WithCoinResult},
        test::coin::Expect,
    };
    use sdk::{
        cosmwasm_std::{coin as cw_coin, Addr, Coin as CwCoin, Empty, QuerierWrapper},
        cw_multi_test::BasicApp,
    };

    use crate::{coin_legacy, error::Error};

    use super::{may_received, BankAccountView as _, BankView, ReduceResults as _};

    type TheCurrency = SubGroupTestC1;
    type ExtraCurrency = SuperGroupTestC1;

    const AMOUNT: Amount = 42;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
    #[error("Test error")]
    struct TestError;

    #[test]
    fn reduce_results_empty() {
        assert_eq!(
            [Ok::<(), TestError>(()); 0]
                .into_iter()
                .reduce_results(|(), ()| unreachable!()),
            None
        );
    }

    #[test]
    fn reduce_results_1_ok() {
        assert_eq!(
            [Ok::<u8, TestError>(1)]
                .into_iter()
                .reduce_results(|_, _| unreachable!()),
            Some(Ok(1))
        );
    }

    #[test]
    fn reduce_results_3_ok() {
        assert_eq!(
            [Ok::<u8, TestError>(1), Ok(2), Ok(3)]
                .into_iter()
                .reduce_results(|acc, element| acc + element),
            Some(Ok(6))
        );
    }

    #[test]
    fn reduce_results_1_err() {
        assert_eq!(
            [Err::<u8, TestError>(TestError)]
                .into_iter()
                .reduce_results(|_, _| unreachable!()),
            Some(Err(TestError))
        );
    }

    #[test]
    fn reduce_results_1_ok_1_err() {
        assert_eq!(
            [Ok::<u8, TestError>(1), Err(TestError)]
                .into_iter()
                .reduce_results(|_, _| unreachable!()),
            Some(Err(TestError))
        );
    }

    #[test]
    fn reduce_results_1_err_1_ok() {
        assert_eq!(
            [Err::<u8, TestError>(TestError), Ok(2)]
                .into_iter()
                .reduce_results(|_, _| unreachable!()),
            Some(Err(TestError))
        );
    }

    #[test]
    fn reduce_results_2_ok_1_err_1_ok() {
        assert_eq!(
            [Ok::<u8, TestError>(1), Ok(2), Err(TestError), Ok(4)]
                .into_iter()
                .reduce_results(|acc, element| {
                    assert_ne!(element, 4);

                    acc + element
                }),
            Some(Err(TestError))
        );
    }

    #[test]
    fn may_received_no_input() {
        assert_eq!(
            None,
            may_received(&vec![], Expect(Coin::<TheCurrency>::from(AMOUNT)))
        );
    }

    #[test]
    fn may_received_not_in_group() {
        let coin_1 = Coin::<ExtraCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm(coin_1);

        let coin_2 = Coin::<ExtraCurrency>::new(AMOUNT + AMOUNT);
        let in_coin_2 = coin_legacy::to_cosmwasm(coin_2);

        assert_eq!(
            None,
            may_received(
                &vec![in_coin_1, in_coin_2],
                Expect(Coin::<TheCurrency>::new(AMOUNT))
            )
        );
    }

    #[test]
    fn may_received_in_group() {
        let coin = Coin::<TheCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm(coin);
        assert_eq!(Some(Ok(true)), may_received(&vec![in_coin_1], Expect(coin)));
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
            may_received(
                &vec![in_coin_1.clone(), in_coin_2.clone(), in_coin_3.clone()],
                Expect(coin_2)
            )
        );
        assert_eq!(
            Some(Ok(true)),
            may_received(&vec![in_coin_1, in_coin_3, in_coin_2], Expect(coin_3),)
        );
    }

    #[derive(Clone)]
    struct Cmd<G>
    where
        G: Group,
    {
        expected: Option<CurrencyDTO<G>>,
    }

    impl<G> Cmd<G>
    where
        G: Group,
    {
        pub fn expected<C>() -> Self
        where
            C: Currency + MemberOf<G>,
        {
            Self {
                expected: Some(currency::dto::<C, G>()),
            }
        }

        pub const fn expected_none() -> Self {
            Self { expected: None }
        }

        fn validate(&self, balances_result: Option<Result<(), Error>>)
        where
            G: Group,
        {
            if self.expected.is_some() {
                assert_eq!(Some(Ok(())), balances_result)
            } else {
                assert_eq!(None, balances_result)
            }
        }
    }

    impl<G> WithCoin<G> for Cmd<G>
    where
        G: Group,
    {
        type VisitorG = G;
        type Output = ();
        type Error = Error;

        fn on<C>(self, _: Coin<C>) -> WithCoinResult<G, Self>
        where
            C: Currency + MemberOf<G>,
        {
            assert_eq!(Some(currency::dto::<C, G>()), self.expected);

            Ok(())
        }
    }

    fn total_balance_tester<G>(coins: Vec<CwCoin>, mock: Cmd<G>)
    where
        G: Group,
    {
        let addr: Addr = Addr::unchecked("user");

        let app: BasicApp<Empty, Empty> = sdk::cw_multi_test::App::new(|router, _, storage| {
            router.bank.init_balance(storage, &addr, coins).unwrap();
        });
        let querier: QuerierWrapper<'_> = app.wrap();

        let bank_view: BankView<'_> = BankView::account(&addr, querier);

        let result = bank_view.balances::<G, _>(mock.clone()).unwrap();
        mock.validate(result);
    }

    #[test]
    fn total_balance_empty() {
        total_balance_tester::<SubGroup>(vec![], Cmd::expected_none());
    }

    #[test]
    fn total_balance_same_group() {
        total_balance_tester::<SubGroup>(
            vec![cw_coin(100, SubGroupTestC1::BANK_SYMBOL)],
            Cmd::expected::<SubGroupTestC1>(),
        );
    }

    #[test]
    fn total_balance_different_group() {
        total_balance_tester::<SubGroup>(
            vec![cw_coin(100, SuperGroupTestC1::BANK_SYMBOL)],
            Cmd::expected_none(),
        );
    }

    #[test]
    fn total_balance_mixed_group() {
        total_balance_tester::<SubGroup>(
            vec![
                cw_coin(100, SuperGroupTestC1::TICKER),
                cw_coin(100, SubGroupTestC1::BANK_SYMBOL),
            ],
            Cmd::expected::<SubGroupTestC1>(),
        );
    }
}
