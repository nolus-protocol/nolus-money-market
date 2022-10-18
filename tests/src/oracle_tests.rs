use std::collections::HashSet;

use currency::lpn::Usdc;
use finance::{
    coin::{Amount, Coin},
    currency::Currency as _,
    percent::Percent,
    price::{self, dto::PriceDTO},
    test::currency::Nls,
};
use leaser::msg::QueryMsg;
use oracle::msg::QueryMsg as OracleQ;
use platform::coin_legacy::to_cosmwasm;
use sdk::{
    cosmwasm_std::{coins, wasm_execute, Addr, Event},
    cw_multi_test::Executor,
};

use crate::common::{
    leaser_wrapper::LeaserWrapper, test_case::TestCase, AppExt, ADMIN, NATIVE_DENOM,
};

type Currency = Usdc;
type TheCoin = Coin<Currency>;
const DENOM: &str = Usdc::TICKER;

fn create_coin(amount: u128) -> TheCoin {
    Coin::<Currency>::new(amount)
}

fn create_test_case() -> TestCase {
    let mut test_case =
        TestCase::with_reserve(DENOM, &coins(10_000_000_000_000_000_000_000_000_000, DENOM));
    test_case.init(
        &Addr::unchecked(ADMIN),
        vec![to_cosmwasm(create_coin(1_000_000_000_000_000_000_000_000))],
    );
    test_case.init_lpp_with_funds(None, 5_000_000_000_000_000_000_000_000_000, DENOM);
    test_case.init_timealarms_with_funds(5_000_000);
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
                vec![to_cosmwasm(create_coin(1000))],
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
                vec![to_cosmwasm(create_coin(1000))],
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
            Addr::unchecked(ADMIN),
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
                owner: Addr::unchecked(ADMIN),
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

    test_case.send_funds(
        &test_case.profit_addr.clone().unwrap(),
        coins(500, NATIVE_DENOM),
    );

    let resp = test_case
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
        .unwrap();

    resp.assert_event(&Event::new("wasm").add_attribute("alarm", "success"))
}

#[test]
fn test_config_update() {
    let mut test_case = create_test_case();

    let admin = Addr::unchecked(ADMIN);
    let feeder1 = Addr::unchecked("feeder1");
    let feeder2 = Addr::unchecked("feeder2");
    let feeder3 = Addr::unchecked("feeder3");
    let base = 2;
    let quote = 10;

    fn add_feeder(test_case: &mut TestCase, addr: impl Into<String>) {
        test_case
            .app
            .execute(
                Addr::unchecked(ADMIN),
                wasm_execute(
                    test_case.oracle.clone().unwrap(),
                    &oracle::msg::ExecuteMsg::RegisterFeeder {
                        feeder_address: addr.into(),
                    },
                    vec![],
                )
                .unwrap()
                .into(),
            )
            .unwrap();
    }

    fn add_price(test_case: &mut TestCase, addr: &Addr, base: Amount, quote: Amount) {
        test_case
            .app
            .execute(
                addr.clone(),
                wasm_execute(
                    test_case.oracle.clone().unwrap(),
                    &oracle::msg::ExecuteMsg::FeedPrices {
                        prices: vec![PriceDTO::try_from(
                            price::total_of(Coin::<Nls>::new(base)).is(Coin::<Usdc>::new(quote)),
                        )
                        .unwrap()],
                    },
                    vec![],
                )
                .unwrap()
                .into(),
            )
            .expect("Oracle not properly connected!");
    }

    add_feeder(&mut test_case, &feeder1);
    add_feeder(&mut test_case, &feeder2);
    add_feeder(&mut test_case, &feeder3);

    add_price(&mut test_case, &feeder1, base, quote);
    add_price(&mut test_case, &feeder2, base, quote);

    let price: PriceDTO = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.oracle.clone().unwrap(),
            &OracleQ::Price {
                currency: Nls::TICKER.to_owned(),
            },
        )
        .unwrap();

    assert_eq!(
        price,
        PriceDTO::try_from(price::total_of(Coin::<Nls>::new(base)).is(Coin::<Usdc>::new(quote)),)
            .unwrap()
    );

    test_case
        .app
        .execute(
            admin,
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::Config {
                    price_feed_period_secs: 60,
                    expected_feeders: Percent::from_percent(100),
                },
                vec![],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!");

    let price: Result<PriceDTO, _> = test_case.app.wrap().query_wasm_smart(
        test_case.oracle.clone().unwrap(),
        &OracleQ::Price {
            currency: Nls::TICKER.to_owned(),
        },
    );

    assert!(price.is_err());
}
