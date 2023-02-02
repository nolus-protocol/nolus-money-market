use std::collections::HashSet;

use currency::{
    lease::{Atom, Cro, Osmo, Wbtc, Weth},
    lpn::Usdc,
};
use finance::{
    coin::Coin,
    currency::Currency,
    duration::Duration,
    percent::Percent,
    price::{self, dto::PriceDTO},
};
use leaser::msg::QueryMsg;
use marketprice::{config::Config as PriceConfig, SpotPrice};
use oracle::{alarms::Alarm, msg::QueryMsg as OracleQ};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{coin, wasm_execute, Addr, Coin as CwCoin, Event, Timestamp},
    cw_multi_test::Executor,
    schemars::_serde_json::from_str,
};
use swap::SwapTarget;
use tree::HumanReadableTree;

use crate::common::{
    leaser_wrapper::LeaserWrapper, native_cwcoin, oracle_wrapper, test_case::TestCase, AppExt,
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

type Lpn = Usdc;
type LeaseCurrency = Atom;
type TheCoin = Coin<Lpn>;
type BaseC = Osmo;

fn cw_coin<CoinT>(coin: CoinT) -> CwCoin
where
    CoinT: Into<Coin<Lpn>>,
{
    coin_legacy::to_cosmwasm(coin.into())
}

fn create_test_case() -> TestCase<Lpn> {
    let mut test_case =
        TestCase::with_reserve(None, &[cw_coin(10_000_000_000_000_000_000_000_000_000)]);
    test_case.init(
        &Addr::unchecked(ADMIN),
        vec![cw_coin(1_000_000_000_000_000_000_000_000)],
    );
    test_case.init_lpp_with_funds(
        None,
        vec![coin(
            5_000_000_000_000_000_000_000_000_000,
            Lpn::BANK_SYMBOL,
        )],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    test_case
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

    let _ = oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        &Addr::unchecked(ADMIN),
        Coin::new(5),
        Coin::new(7),
    );
}

// test for issue #26. It was resolved in MR !132 by separation of price feeding and alarms delivery processes
#[test]
fn feed_price_with_alarm_issue() {
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

    let lease = open_lease(&mut test_case, Coin::new(1000));

    // there is no price in the oracle and feed for this alarm
    test_case
        .app
        .execute_contract(
            lease,
            test_case.oracle.clone().unwrap(),
            &oracle::msg::ExecuteMsg::AddPriceAlarm {
                alarm: Alarm::new(
                    price::total_of(Coin::<Cro>::new(1)).is(Coin::<Usdc>::new(1)),
                    None,
                ),
            },
            &[],
        )
        .unwrap();

    let _ = oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        &Addr::unchecked(ADMIN),
        Coin::new(5),
        Coin::new(7),
    );
}

#[test]
fn feed_price_with_alarm() {
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

    let lease = open_lease(&mut test_case, Coin::new(1000));

    test_case
        .app
        .execute_contract(
            lease,
            test_case.oracle.clone().unwrap(),
            &oracle::msg::ExecuteMsg::AddPriceAlarm {
                alarm: Alarm::new(
                    price::total_of(Coin::<Cro>::new(1)).is(Coin::<Usdc>::new(10)),
                    None,
                ),
            },
            &[],
        )
        .unwrap();

    let res = oracle_wrapper::feed_price::<_, Cro, Usdc>(
        &mut test_case,
        &Addr::unchecked(ADMIN),
        Coin::new(1),
        Coin::new(5),
    );

    dbg!(res);
}

fn open_lease(test_case: &mut TestCase<Lpn>, value: TheCoin) -> Addr {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LeaseCurrency::TICKER.into(),
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

    let resp = test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.timealarms.unwrap(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
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

    oracle_wrapper::add_feeder(&mut test_case, &feeder1);
    oracle_wrapper::add_feeder(&mut test_case, &feeder2);
    oracle_wrapper::add_feeder(&mut test_case, &feeder3);

    oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        &feeder1,
        Coin::new(base),
        Coin::new(quote),
    );
    oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        &feeder2,
        Coin::new(base),
        Coin::new(quote),
    );

    let price: SpotPrice = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.oracle.clone().unwrap(),
            &OracleQ::Price {
                currency: BaseC::TICKER.into(),
            },
        )
        .unwrap();

    assert_eq!(
        price,
        PriceDTO::try_from(price::total_of(Coin::<BaseC>::new(base)).is(Coin::<Usdc>::new(quote)))
            .unwrap()
    );

    test_case
        .app
        .execute(
            admin,
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::UpdateConfig(PriceConfig::new(
                    Percent::from_percent(100),
                    Duration::from_secs(5),
                    12,
                    Percent::from_percent(75),
                )),
                vec![],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!");

    let price: Result<SpotPrice, _> = test_case.app.wrap().query_wasm_smart(
        test_case.oracle.clone().unwrap(),
        &OracleQ::Price {
            currency: BaseC::TICKER.into(),
        },
    );

    assert!(price.is_err());
}

fn swap_tree() -> HumanReadableTree<SwapTarget> {
    serde_json_wasm::from_str(&format!(
        r#"{{
                "value":[0,"{usdc}"],
                "children":[
                    {{
                        "value":[1,"{base_c}"],
                        "children":[
                            {{"value":[2,"{weth}"]}},
                            {{"value":[3,"{wbtc}"]}}
                        ]
                    }}
                ]
            }}"#,
        usdc = Usdc::TICKER,
        base_c = BaseC::TICKER,
        weth = Weth::TICKER,
        wbtc = Wbtc::TICKER,
    ))
    .unwrap()
}

#[test]
fn test_swap_path() {
    let mut test_case = create_test_case();
    let admin = Addr::unchecked(ADMIN);
    let msg = oracle::msg::ExecuteMsg::SwapTree { tree: swap_tree() };
    test_case
        .app
        .execute_contract(admin, test_case.oracle.clone().unwrap(), &msg, &[])
        .unwrap();
    let resp: swap::SwapPath = test_case
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

#[test]
fn test_query_swap_tree() {
    let mut test_case = create_test_case();
    let admin = Addr::unchecked(ADMIN);
    let tree: HumanReadableTree<SwapTarget> = swap_tree();
    let msg = oracle::msg::ExecuteMsg::SwapTree { tree: tree.clone() };
    test_case
        .app
        .execute_contract(admin, test_case.oracle.clone().unwrap(), &msg, &[])
        .unwrap();
    let resp: oracle::msg::SwapTreeResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(test_case.oracle.unwrap(), &OracleQ::SwapTree {})
        .unwrap();

    assert_eq!(tree, resp.tree);
}

#[test]
#[should_panic]
fn test_zero_price_dto() {
    let mut test_case = create_test_case();

    let feeder1 = Addr::unchecked("feeder1");

    oracle_wrapper::add_feeder(&mut test_case, &feeder1);

    // can be created only via deserialization
    let price: SpotPrice = from_str(
        r#"{"amount":{"amount":0,"ticker":"OSMO"},"amount_quote":{"amount":1,"ticker":"USDC"}}"#,
    )
    .unwrap();

    test_case
        .app
        .execute(
            feeder1,
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &oracle::msg::ExecuteMsg::FeedPrices {
                    prices: vec![price],
                },
                vec![],
            )
            .unwrap()
            .into(),
        )
        .unwrap();
}
