use std::collections::HashSet;

use cosmwasm_std::{wasm_execute, Addr};
use cw_multi_test::Executor;

use finance::{
    coin::Coin,
    currency::{Nls, Usdc},
    price::{self, dto::PriceDTO},
};
use leaser::msg::QueryMsg;
use platform::coin_legacy::to_cosmwasm;

use crate::common::{leaser_wrapper::LeaserWrapper, test_case::TestCase, AppExt, ADMIN, USER};

type Currency = Usdc;
type TheCoin = Coin<Currency>;
const DENOM: &str = <Usdc as finance::currency::Currency>::SYMBOL;

fn create_coin(amount: u128) -> TheCoin {
    Coin::<Currency>::new(amount)
}

fn create_test_case() -> TestCase {
    let mut test_case = TestCase::with_reserve(DENOM, 10_000_000_000);
    test_case.init(
        &Addr::unchecked("user"),
        vec![to_cosmwasm(create_coin(1_000_000))],
    );
    test_case.init_lpp_with_funds(None, 5_000_000_000);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    test_case
}

#[test]
fn internal_test_integration_setup_test() {
    let mut test_case = create_test_case();

    test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::RegisterFeeder {
                    feeder_address: ADMIN.into(),
                },
                vec![to_cosmwasm(create_coin(10000))],
            )
            .unwrap()
            .into(),
        )
        .unwrap();

    test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::FeedPrices {
                    prices: vec![PriceDTO::try_from(
                        price::total_of(Coin::<Nls>::new(5)).is(Coin::<Usdc>::new(7)),
                    )
                    .unwrap()],
                },
                vec![to_cosmwasm(create_coin(10000))],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!");
}

fn open_lease(test_case: &mut TestCase, value: TheCoin) -> Addr {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(USER),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: DENOM.to_string(),
            },
            &[to_cosmwasm(value)],
        )
        .unwrap();

    get_lease_address(test_case)
}

fn get_lease_address(test_case: &TestCase) -> Addr {
    let query_response: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases {
                owner: Addr::unchecked(USER),
            },
        )
        .unwrap();
    assert_eq!(query_response.len(), 1);
    query_response.iter().next().unwrap().clone()
}

#[test]
fn integration_with_timealarms() {
    let mut test_case = create_test_case();

    test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::RegisterFeeder {
                    feeder_address: ADMIN.into(),
                },
                vec![to_cosmwasm(create_coin(10000))],
            )
            .unwrap()
            .into(),
        )
        .unwrap();

    let _lease = open_lease(&mut test_case, create_coin(1_000));

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD + LeaserWrapper::GRACE_PERIOD,
    );

    test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::FeedPrices {
                    prices: vec![PriceDTO::try_from(
                        price::total_of(Coin::<Nls>::new(5)).is(Coin::<Usdc>::new(7)),
                    )
                    .unwrap()],
                },
                vec![to_cosmwasm(create_coin(10000))],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!");
}
