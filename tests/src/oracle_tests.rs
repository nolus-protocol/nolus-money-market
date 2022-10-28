use std::collections::HashSet;

use currency::{
    lease::{Osmo, Wbtc, Weth},
    lpn::Usdc,
};
use finance::{
    coin::{Amount, Coin},
    currency::Currency as _,
    percent::Percent,
    price::{self, dto::PriceDTO},
};
use leaser::msg::QueryMsg;
use oracle::{
    msg::QueryMsg as OracleQ,
    state::supported_pairs::{SwapTarget, TreeStore},
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{wasm_execute, Addr, Coin as CwCoin, Event, Timestamp},
    cw_multi_test::{AppResponse, Executor},
};

use crate::common::{
    leaser_wrapper::LeaserWrapper, native_cwcoin, test_case::TestCase, AppExt, ADMIN, USER,
};

use trees::tr;

type Lpn = Usdc;
type TheCoin = Coin<Lpn>;
type BaseC = Osmo;

fn cw_coin<CoinT>(coin: CoinT) -> CwCoin
where
    CoinT: Into<Coin<Lpn>>,
{
    coin_legacy::to_cosmwasm(coin.into())
}

fn create_test_case() -> TestCase<Lpn> {
    let mut test_case = TestCase::with_reserve(&[cw_coin(10_000_000_000_000_000_000_000_000_000)]);
    test_case.init(
        &Addr::unchecked(ADMIN),
        vec![cw_coin(1_000_000_000_000_000_000_000_000)],
    );
    test_case.init_lpp_with_funds(None, 5_000_000_000_000_000_000_000_000_000.into());
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    test_case
}

fn feed_price(
    test_case: &mut TestCase<Lpn>,
    addr: &Addr,
    base: Amount,
    quote: Amount,
) -> AppResponse {
    test_case
        .app
        .execute(
            addr.clone(),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::FeedPrices {
                    prices: vec![PriceDTO::try_from(
                        price::total_of(Coin::<BaseC>::new(base)).is(Coin::<Usdc>::new(quote)),
                    )
                    .unwrap()],
                },
                vec![],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!")
}

#[test]
fn register_feeder() {
    let mut test_case = create_test_case();
    let user = Addr::unchecked(USER);
    let admin = Addr::unchecked(ADMIN);

    // only admin can register new feeder, other user should result in error
    let msg = oracle::msg::ExecuteMsg::RegisterFeeder {
        feeder_address: USER.to_string(),
    };
    test_case
        .app
        .execute_contract(user, test_case.oracle.clone().unwrap(), &msg, &[])
        .unwrap_err();

    // check if admin can register new feeder
    let msg = oracle::msg::ExecuteMsg::RegisterFeeder {
        feeder_address: ADMIN.to_string(),
    };
    test_case
        .app
        .execute_contract(admin, test_case.oracle.clone().unwrap(), &msg, &[])
        .unwrap();
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
                vec![cw_coin(1000)],
            )
            .unwrap()
            .into(),
        )
        .unwrap();

    let _ = feed_price(&mut test_case, &Addr::unchecked(ADMIN), 5, 7);
}

fn open_lease(test_case: &mut TestCase<Lpn>, value: TheCoin) -> Addr {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: Lpn::TICKER.into(),
            },
            &[cw_coin(value)],
        )
        .unwrap();

    get_lease_address(test_case)
}

fn get_lease_address(test_case: &TestCase<Lpn>) -> Addr {
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
#[should_panic]
fn wrong_timealarms_addr() {
    let mut test_case = create_test_case();

    let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
        time: Timestamp::from_seconds(100),
    };

    test_case
        .app
        .execute_contract(
            Addr::unchecked(USER),
            test_case.oracle.clone().unwrap(),
            &alarm_msg,
            &[],
        )
        .unwrap();
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
                vec![cw_coin(10000)],
            )
            .unwrap()
            .into(),
        )
        .unwrap();

    let _lease = open_lease(&mut test_case, TheCoin::from(1_000));

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD + LeaserWrapper::GRACE_PERIOD,
    );

    test_case.send_funds(
        &test_case.profit_addr.clone().unwrap(),
        vec![native_cwcoin(500)],
    );

    let resp = feed_price(&mut test_case, &Addr::unchecked(ADMIN), 5, 7);

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

    fn add_feeder(test_case: &mut TestCase<Lpn>, addr: impl Into<String>) {
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

    add_feeder(&mut test_case, &feeder1);
    add_feeder(&mut test_case, &feeder2);
    add_feeder(&mut test_case, &feeder3);

    feed_price(&mut test_case, &feeder1, base, quote);
    feed_price(&mut test_case, &feeder2, base, quote);

    let price: PriceDTO = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.oracle.clone().unwrap(),
            &OracleQ::Price {
                currency: BaseC::TICKER.to_owned(),
            },
        )
        .unwrap();

    assert_eq!(
        price,
        PriceDTO::try_from(price::total_of(Coin::<BaseC>::new(base)).is(Coin::<Usdc>::new(quote)),)
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
            currency: BaseC::TICKER.to_owned(),
        },
    );

    assert!(price.is_err());
}

#[test]
fn test_swap_path() {
    let mut test_case = create_test_case();
    let admin = Addr::unchecked(ADMIN);
    let msg = oracle::msg::ExecuteMsg::SwapTree {
        tree: TreeStore(
            tr((0, Usdc::TICKER.into()))
                / (tr((1, BaseC::TICKER.to_string()))
                    / tr((2, Weth::TICKER.to_string()))
                    / tr((3, Wbtc::TICKER.to_string()))),
        ),
    };
    test_case
        .app
        .execute_contract(admin, test_case.oracle.clone().unwrap(), &msg, &[])
        .unwrap();
    let resp: oracle::msg::SwapPathResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.oracle.unwrap(),
            &OracleQ::SwapPath {
                from: Wbtc::TICKER.into(),
                to: Weth::TICKER.into(),
            },
        )
        .unwrap();

    let expect = vec![
        SwapTarget {
            pool_id: 3,
            target: BaseC::TICKER.into(),
        },
        SwapTarget {
            pool_id: 2,
            target: Weth::TICKER.into(),
        },
    ];

    assert_eq!(resp, expect);
}
