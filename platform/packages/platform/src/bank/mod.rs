pub use account::{BankAccount, account};
pub use aggregate::Aggregate;
pub use receive::{may_received, received_one};
#[cfg(any(test, feature = "testing"))]
pub use send::bank_send;
pub use send::{FixedAddressSender, LazySenderStub, bank_send_all};
pub use view::{BalancesResult, BankAccountView, account_view, balance, cache};

mod account;
mod aggregate;
mod receive;
mod send;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
mod view;

#[cfg(test)]
mod test {

    use currency::{
        CurrencyDTO, CurrencyDef, Group, MemberOf,
        test::{
            SubGroup, SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1,
            SuperGroupTestC4,
        },
    };
    use finance::{
        coin::{Amount, Coin, WithCoin},
        test::coin::Expect,
    };
    use sdk::{
        cosmwasm_std::{Addr, Coin as CwCoin, QuerierWrapper, coin as cw_coin},
        cw_multi_test::BasicApp,
        testing,
    };

    use crate::{
        bank::{
            aggregate::ReduceResults,
            receive, send,
            view::{self, BankAccountView},
        },
        coin_legacy,
    };

    type TheCurrency = SubGroupTestC10;
    type ExtraCurrency = SuperGroupTestC1;

    const AMOUNT: Amount = 42;
    const USER: &str = "user";

    #[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
    #[error("Test error")]
    struct TestError;

    #[test]
    fn reduce_results_empty() {
        assert_eq!(
            [const { Ok::<(), TestError>(()) }; 0]
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
            receive::may_received(&vec![], Expect(Coin::<TheCurrency>::from(AMOUNT)))
        );
    }

    #[test]
    fn may_received_not_in_group() {
        let coin_1 = Coin::<ExtraCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm_on_nolus(coin_1);

        let coin_2 = Coin::<ExtraCurrency>::new(AMOUNT + AMOUNT);
        let in_coin_2 = coin_legacy::to_cosmwasm_on_nolus(coin_2);

        assert_eq!(
            None,
            receive::may_received(
                &vec![in_coin_1, in_coin_2],
                Expect(Coin::<TheCurrency>::new(AMOUNT))
            )
        );
    }

    #[test]
    fn may_received_in_group() {
        let coin = Coin::<TheCurrency>::new(AMOUNT);
        let in_coin_1 = coin_legacy::to_cosmwasm_on_nolus(coin);
        assert_eq!(
            Some(true),
            receive::may_received(&vec![in_coin_1], Expect(coin))
        );
    }

    #[test]
    fn may_received_in_group_others_arround() {
        let in_coin_1 =
            coin_legacy::to_cosmwasm_on_nolus(Coin::<ExtraCurrency>::new(AMOUNT + AMOUNT));

        let coin_2 = Coin::<TheCurrency>::new(AMOUNT);
        let in_coin_2 = coin_legacy::to_cosmwasm_on_nolus(coin_2);

        let coin_3 = Coin::<TheCurrency>::new(AMOUNT + AMOUNT);
        let in_coin_3 = coin_legacy::to_cosmwasm_on_nolus(coin_3);
        assert_eq!(
            Some(true),
            receive::may_received(
                &vec![in_coin_1.clone(), in_coin_2.clone(), in_coin_3.clone()],
                Expect(coin_2)
            )
        );
        assert_eq!(
            Some(true),
            receive::may_received(&vec![in_coin_1, in_coin_3, in_coin_2], Expect(coin_3),)
        );
    }

    #[derive(Clone)]
    struct Cmd<G>
    where
        G: 'static + Group,
    {
        expected: Option<&'static CurrencyDTO<G>>,
    }

    impl<G> Cmd<G>
    where
        G: Group,
    {
        pub fn expected<C>() -> Cmd<G>
        where
            C: CurrencyDef<Group = G>,
        {
            Cmd::<C::Group> {
                expected: Some(C::dto()),
            }
        }

        pub const fn expected_none() -> Self {
            Self { expected: None }
        }

        fn validate(&self, balances_result: Option<()>)
        where
            G: Group,
        {
            if self.expected.is_some() {
                assert_eq!(Some(()), balances_result)
            } else {
                assert_eq!(None, balances_result)
            }
        }
    }

    impl<G> WithCoin<G> for Cmd<G>
    where
        G: Group,
    {
        type Outcome = ();

        fn on<C>(self, _: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G>,
        {
            assert_eq!(Some(&C::dto().into_super_group::<G>()), self.expected);
        }
    }

    fn total_balance_tester<G>(coins: Vec<CwCoin>, mock: Cmd<G>)
    where
        G: Group,
    {
        let user = testing::user(USER);

        let app = BasicApp::new(|router, _, storage| {
            router.bank.init_balance(storage, &user, coins).unwrap();
        });
        let querier: QuerierWrapper<'_> = app.wrap();

        let bank_view = view::account_view(&user, querier);

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
            vec![cw_coin(100, SubGroupTestC10::bank())],
            Cmd::<SubGroup>::expected::<SubGroupTestC10>(),
        );
    }

    #[test]
    fn total_balance_different_group() {
        total_balance_tester::<SubGroup>(
            vec![cw_coin(100, SuperGroupTestC1::bank())],
            Cmd::expected_none(),
        );
    }

    #[test]
    fn total_balance_mixed_group() {
        total_balance_tester::<SubGroup>(
            vec![
                cw_coin(100, SuperGroupTestC1::ticker()),
                cw_coin(100, SubGroupTestC10::bank()),
            ],
            Cmd::<SubGroup>::expected::<SubGroupTestC10>(),
        );
    }

    #[test]
    fn send_all_none() {
        send_all_tester::<SuperGroup>(vec![], 0);
    }

    #[test]
    fn send_all_subgroup() {
        send_all_tester::<SubGroup>(vec![cw_coin(200, SuperGroupTestC4::bank())], 0);

        send_all_tester::<SubGroup>(vec![cw_coin(100, SubGroupTestC10::dex())], 0);

        send_all_tester::<SubGroup>(
            vec![
                cw_coin(100, SuperGroupTestC1::ticker()),
                cw_coin(100, SubGroupTestC10::bank()),
                cw_coin(200, SuperGroupTestC4::bank()),
            ],
            1,
        );
    }

    #[test]
    fn send_all_supergroup() {
        send_all_tester::<SuperGroup>(vec![cw_coin(200, SuperGroupTestC4::dex())], 0);

        send_all_tester::<SuperGroup>(vec![cw_coin(100, SubGroupTestC10::bank())], 1);

        send_all_tester::<SuperGroup>(
            vec![
                cw_coin(100, SubGroupTestC10::bank()),
                cw_coin(100, SuperGroupTestC1::ticker()),
                cw_coin(100, SubGroupTestC6::bank()),
                cw_coin(200, SuperGroupTestC4::bank()),
            ],
            3,
        );
    }

    #[track_caller]
    fn send_all_tester<G>(coins: Vec<CwCoin>, exp_coins_nb: usize)
    where
        G: Group,
    {
        let from: Addr = testing::user(USER);
        let to = from.clone();

        let app = BasicApp::new(|router, _, storage| {
            router.bank.init_balance(storage, &from, coins).unwrap();
        });
        let querier: QuerierWrapper<'_> = app.wrap();

        let msgs = send::bank_send_all::<G>(&from, to, querier).unwrap();
        assert_eq!(exp_coins_nb, msgs.len());
    }
}
