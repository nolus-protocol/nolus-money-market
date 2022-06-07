use cosmwasm_std::{coins, Addr};
use cw_multi_test::{ContractWrapper, Executor};

use crate::{
    common::test_case::TestCase,
    common::{
        lpp_wrapper::{mock_lpp_execute, mock_lpp_query},
        oracle_wrapper::mock_oracle_query,
        ADMIN, USER,
    },
};

#[test]
fn on_alarm_zero_reeward() {
    let denom = "UST";
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));

    test_case
        .init_lpp(None)
        .init_market_oracle(None)
        .init_time_oracle()
        .init_treasury()
        .init_dispatcher();

    let res = test_case
        .app
        .execute_contract(
            test_case.time_oracle.unwrap(),
            test_case.dispatcher_addr.as_ref().unwrap().clone(),
            &rewards_dispatcher::msg::ExecuteMsg::Alarm {
                time: test_case.app.block_info().time,
            },
            &coins(40, denom),
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(2, res.events.len(), "{:?}", res.events);
    let dispatcher_exec = &res.events[0];
    assert_eq!(dispatcher_exec.ty.as_str(), "execute");
    assert_eq!(
        dispatcher_exec.attributes,
        [(
            "_contract_addr",
            test_case.dispatcher_addr.as_ref().unwrap()
        )]
    );
    let dispatcher_exec = &res.events[1];
    assert_eq!(dispatcher_exec.ty.as_str(), "wasm");
    assert_eq!(
        dispatcher_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.dispatcher_addr.unwrap().to_string()
            ),
            ("method", "try_dispatch".to_string()),
            ("result", "no reward to dispatch".to_string())
        ]
    );
}

#[test]
fn on_alarm() {
    let denom = "UST";
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));
    assert_eq!(
        coins(500, denom),
        test_case
            .app
            .wrap()
            .query_all_balances(time_oracle_addr)
            .unwrap()
    );

    test_case
        .init_lpp(Some(ContractWrapper::new(
            mock_lpp_execute,
            lpp::contract::instantiate,
            mock_lpp_query,
        )))
        .init_market_oracle(Some(ContractWrapper::new(
            oracle::contract::execute,
            oracle::contract::instantiate,
            mock_oracle_query,
        )))
        .init_time_oracle()
        .init_treasury()
        .init_dispatcher();

    let res = test_case
        .app
        .execute_contract(
            Addr::unchecked("time"),
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::ExecuteMsg::Alarm {
                time: test_case.app.block_info().time,
            },
            &coins(40, denom),
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(5, res.events.len(), "{:?}", res.events);
    // reflect only returns standard wasm-execute event
    let dispatcher_exec = &res.events[0];
    assert_eq!(dispatcher_exec.ty.as_str(), "execute");
    assert_eq!(
        dispatcher_exec.attributes,
        [("_contract_addr", test_case.dispatcher_addr.unwrap())]
    );
    let treasury_exec = &res.events[1];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", &test_case.treasury_addr.clone().unwrap())]
    );
    let treasury_wasm = &res.events[2];
    assert_eq!(treasury_wasm.ty.as_str(), "wasm");
    assert_eq!(
        treasury_wasm.attributes,
        [
            (
                "_contract_addr",
                test_case.treasury_addr.clone().unwrap().to_string()
            ),
            ("method", "try_send_rewards".to_string())
        ]
    );
    let treasury_wasm = &res.events[3];
    assert_eq!(treasury_wasm.ty.as_str(), "transfer");
    assert_eq!(
        treasury_wasm.attributes,
        [
            ("recipient", test_case.lpp_addr.clone().unwrap().to_string()),
            (
                "sender",
                test_case.treasury_addr.clone().unwrap().to_string()
            ),
            ("amount", "72UST".to_string())
        ]
    );
    let treasury_exec = &res.events[4];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", &test_case.lpp_addr.unwrap())]
    );
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_config_unauthorized() {
    let denom = "UST";
    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::new(denom);
    test_case
        .init(&user_addr, coins(500, denom))
        .init_lpp(None)
        .init_treasury()
        .init_market_oracle(None)
        .init_time_oracle()
        .init_dispatcher();

    let resp: rewards_dispatcher::msg::ConfigResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(10, resp.cadence_hours);

    let _res = test_case
        .app
        .execute_contract(
            user_addr,
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::ExecuteMsg::Config { cadence_hours: 30 },
            &coins(40, denom),
        )
        .unwrap();
}

#[test]
fn test_config() {
    let denom = "UST";
    let user_addr = Addr::unchecked(ADMIN);
    let mut test_case = TestCase::new(denom);
    test_case
        .init(&user_addr, coins(500, denom))
        .init_lpp(None)
        .init_treasury()
        .init_market_oracle(None)
        .init_time_oracle()
        .init_dispatcher();

    let resp: rewards_dispatcher::msg::ConfigResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(10, resp.cadence_hours);

    let _res = test_case
        .app
        .execute_contract(
            user_addr,
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::ExecuteMsg::Config { cadence_hours: 30 },
            &coins(40, denom),
        )
        .unwrap();
    let resp: rewards_dispatcher::msg::ConfigResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(30, resp.cadence_hours);
}
