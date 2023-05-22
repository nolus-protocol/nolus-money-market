use std::collections::HashSet;

use serde_json_wasm::from_str;

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
    cw_multi_test::{AppResponse, Executor},
    schemars::_serde_json::from_str,
};
use swap::SwapTarget;
use tree::HumanReadableTree;

use crate::common::{
    oracle_wrapper, test_case::TestCase, ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE,
    USER, UTILIZATION_OPTIMAL,
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
    let mut test_case = TestCase::with_reserve(&[cw_coin(10_000_000_000_000_000_000_000_000_000)]);
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
fn test_lease_serde() {
    use lease::api::ExecuteMsg::PriceAlarm as LeasePriceAlarm;
    use oracle::msg::ExecuteAlarmMsg::PriceAlarm;

    let LeasePriceAlarm {} = serde_json_wasm::from_slice(&serde_json_wasm::to_vec(&PriceAlarm {}).unwrap()).unwrap() else {
        unreachable!()
    };

    let PriceAlarm {} =
        serde_json_wasm::from_slice(&serde_json_wasm::to_vec(&LeasePriceAlarm {}).unwrap())
            .unwrap();
}

#[test]
fn register_feeder() {
    let mut test_case = create_test_case();
    let _user = Addr::unchecked(USER);
    let _admin = Addr::unchecked(ADMIN);

    oracle_wrapper::add_feeder(&mut test_case, ADMIN);
}

#[test]
fn internal_test_integration_setup_test() {
    let mut test_case = create_test_case();

    oracle_wrapper::add_feeder(&mut test_case, ADMIN);

    let response: AppResponse = oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        Addr::unchecked(ADMIN),
        Coin::new(5),
        Coin::new(7),
    );
    assert_eq!(response.data, None);
    assert_eq!(
        &response.events,
        &[Event::new("execute").add_attribute("_contract_addr", "contract2")]
    );
}

// test for issue #26. It was resolved in MR !132 by separation of price feeding and alarms delivery processes
#[test]
fn feed_price_with_alarm_issue() {
    let mut test_case = create_test_case();
    oracle_wrapper::add_feeder(&mut test_case, ADMIN);

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

    test_case.message_receiver.assert_empty();

    let _ = oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        Addr::unchecked(ADMIN),
        Coin::new(5),
        Coin::new(7),
    );
}

#[test]
fn feed_price_with_alarm() {
    let mut test_case = create_test_case();
    oracle_wrapper::add_feeder(&mut test_case, ADMIN);

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

    test_case.message_receiver.assert_empty();

    let res = oracle_wrapper::feed_price::<_, Cro, Usdc>(
        &mut test_case,
        Addr::unchecked(ADMIN),
        Coin::new(1),
        Coin::new(5),
    );

    dbg!(res);
}

fn open_lease(test_case: &mut TestCase<Lpn>, downpayment: TheCoin) -> Addr {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LeaseCurrency::TICKER.into(),
                max_ltd: None,
            },
            &[cw_coin(downpayment)],
        )
        .unwrap();

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

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
fn test_config_update() {
    let mut test_case = create_test_case();

    let _admin = Addr::unchecked(ADMIN);
    let feeder1 = Addr::unchecked("feeder1");
    let feeder2 = Addr::unchecked("feeder2");
    let feeder3 = Addr::unchecked("feeder3");
    let base = 2;
    let quote = 10;

    oracle_wrapper::add_feeder(&mut test_case, &feeder1);
    oracle_wrapper::add_feeder(&mut test_case, &feeder2);
    oracle_wrapper::add_feeder(&mut test_case, &feeder3);

    test_case.message_receiver.assert_empty();

    oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        feeder1,
        Coin::new(base),
        Coin::new(quote),
    );
    oracle_wrapper::feed_price::<_, BaseC, Usdc>(
        &mut test_case,
        feeder2,
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

    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            test_case.oracle.clone().unwrap(),
            &oracle::msg::SudoMsg::UpdateConfig(PriceConfig::new(
                Percent::from_percent(100),
                Duration::from_secs(5),
                12,
                Percent::from_percent(75),
            )),
        )
        .expect("Oracle not properly connected!");
    assert_eq!(response.data, None);
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract2")]
    );

    let price: Result<SpotPrice, _> = test_case.app.wrap().query_wasm_smart(
        test_case.oracle.clone().unwrap(),
        &OracleQ::Price {
            currency: BaseC::TICKER.into(),
        },
    );

    assert!(price.is_err());
}

fn swap_tree() -> HumanReadableTree<SwapTarget> {
    from_str(&format!(
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

    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            test_case.oracle.clone().unwrap(),
            &oracle::msg::SudoMsg::SwapTree { tree: swap_tree() },
        )
        .unwrap();
    assert_eq!(response.data, None);
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract2")]
    );

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

    let tree: HumanReadableTree<SwapTarget> = swap_tree();

    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            test_case.oracle.clone().unwrap(),
            &oracle::msg::SudoMsg::SwapTree { tree: tree.clone() },
        )
        .unwrap();
    assert_eq!(response.data, None);
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract2")]
    );

    let resp: oracle::msg::SwapTreeResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(test_case.oracle.unwrap(), &OracleQ::SwapTree {})
        .unwrap();

    assert_eq!(resp.tree, tree);
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

    let response: AppResponse = test_case
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
    assert_eq!(response.data, None);
    assert_eq!(&response.events, &[]);
}
