use currency::{
    dex::test::{NativeC, StableC1},
    Currency,
};
use rewards_dispatcher::{msg::ConfigResponse, ContractError};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Event},
    cw_multi_test::{AppResponse, ContractWrapper},
};

use crate::common::{
    cwcoin, lpp as lpp_mod, native_cwcoin, oracle as oracle_mod,
    test_case::{builder::BlankBuilder as TestCaseBuilder, TestCase},
    Native, ADDON_OPTIMAL_INTEREST_RATE, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

type Lpn = StableC1;

#[test]
fn on_alarm_zero_reward() {
    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                oracle_mod::mock_query,
            )
            .with_reply(oracle::contract::reply)
            .with_sudo(oracle::contract::sudo),
        ))
        .init_treasury_with_dispatcher(Addr::unchecked("contract4"))
        .init_dispatcher()
        .into_generic();

    test_case.send_funds_from_admin(Addr::unchecked(USER), &[cwcoin::<Lpn, _>(500)]);

    test_case.send_funds_from_admin(
        test_case.address_book.time_alarms().clone(),
        &[cwcoin::<Lpn, _>(500)],
    );

    assert_eq!(test_case.address_book.dispatcher().as_str(), "contract4");

    let err = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.dispatcher().clone(),
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm {},
            &[cwcoin::<Lpn, _>(40)],
        )
        .unwrap_err();
    let root_err = err.root_cause().downcast_ref::<ContractError>();
    assert!(matches!(root_err, Some(&ContractError::ZeroReward {})));
}

#[test]
fn on_alarm() {
    let lender = Addr::unchecked(USER);

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            Some(
                ContractWrapper::new(
                    lpp::contract::execute,
                    lpp::contract::instantiate,
                    lpp_mod::mock_query,
                )
                .with_sudo(lpp::contract::sudo),
            ),
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                oracle_mod::mock_query,
            )
            .with_reply(oracle::contract::reply)
            .with_sudo(oracle::contract::sudo),
        ))
        .init_treasury_without_dispatcher()
        .init_dispatcher()
        .into_generic();

    test_case
        .send_funds_from_admin(
            test_case.address_book.time_alarms().clone(),
            &[cwcoin::<Lpn, _>(500)],
        )
        .send_funds_from_admin(lender.clone(), &[cwcoin::<Lpn, _>(500)]);

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.lpp().clone(), NativeC::TICKER)
            .unwrap(),
        CwCoin::new(0, NativeC::TICKER)
    );

    let treasury_balance = test_case
        .app
        .query()
        .query_all_balances(test_case.address_book.treasury().clone())
        .unwrap();

    println!("treasury_balance = {:?}", treasury_balance);

    // make a deposit to LPP from lenders address
    () = test_case
        .app
        .execute(
            lender.clone(),
            test_case.address_book.lpp().clone(),
            &lpp::msg::ExecuteMsg::Deposit {},
            &[cwcoin::<Lpn, _>(100)],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    // call dispatcher on alarm
    let res: AppResponse = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.dispatcher().clone(),
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .unwrap_response();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(res.events.len(), 6, "{:?}", res.events);

    let dispatcher_exec = &res.events[0];
    assert_eq!(dispatcher_exec.ty, "execute");
    assert_eq!(
        dispatcher_exec.attributes,
        [("_contract_addr", test_case.address_book.dispatcher())]
    );
    let dispatcher_exec = &res.events[1];
    assert_eq!(dispatcher_exec.ty.as_str(), "wasm-tr-rewards");
    assert_eq!(
        dispatcher_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.address_book.dispatcher().as_str()
            ),
            ("height", &test_case.app.block_info().height.to_string()),
            ("at", &test_case.app.block_info().time.nanos().to_string()),
            ("idx", "0"),
            ("to", test_case.address_book.lpp().as_str()),
            ("rewards-amount", "11"),
            ("rewards-symbol", NativeC::TICKER),
        ]
    );
    let treasury_exec = &res.events[2];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", test_case.address_book.treasury())]
    );

    let treasury_exec = &res.events[3];
    assert_eq!(treasury_exec.ty.as_str(), "transfer");
    assert_eq!(
        treasury_exec.attributes,
        [
            ("recipient", test_case.address_book.dispatcher().as_str()),
            ("sender", test_case.address_book.treasury().as_str()),
            ("amount", &format!("11{}", Native::BANK_SYMBOL))
        ]
    );

    let lpp_exec = &res.events[4];
    assert_eq!(lpp_exec.ty.as_str(), "execute");
    assert_eq!(
        lpp_exec.attributes,
        [("_contract_addr", &test_case.address_book.lpp())]
    );

    let time_alarms_exec = &res.events[5];
    assert_eq!(time_alarms_exec.ty.as_str(), "execute");
    assert_eq!(
        time_alarms_exec.attributes,
        [("_contract_addr", &test_case.address_book.time_alarms())]
    );

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.lpp().clone(), Native::BANK_SYMBOL)
            .unwrap(),
        native_cwcoin(11)
    );

    //query calculated reward for the lender
    let resp: lpp::msg::RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &lpp::msg::QueryMsg::Rewards { address: lender },
        )
        .unwrap();

    println!("LPP rewards {:?}", resp);
}

#[test]
fn test_config() {
    let mut test_case = new_test_case();

    let resp = query_config(&test_case);
    assert_eq!(resp.cadence_hours, 10);

    let response: AppResponse = test_case
        .app
        .sudo(
            test_case.address_book.dispatcher().clone(),
            &rewards_dispatcher::msg::SudoMsg::Config { cadence_hours: 30 },
        )
        .unwrap()
        .unwrap_response();
    assert_eq!(response.data, None);
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract4"),]
    );

    let resp = query_config(&test_case);
    assert_eq!(resp.cadence_hours, 30);
}

// TODO: moved from contract tests, should be implemented as integration test
// #[test]
// fn dispatch_with_valid_period() {
//     // let lpp_stub = LppLocalStubUnreachable {};

//     let native_denom = Nls::SYMBOL;
//     let mut deps = mock_dependencies_with_balance(&coins(20, native_denom));
//     do_instantiate(deps.as_mut());

//     let mut env = mock_env();
//     env.block = BlockInfo {
//         height: 12_345,
//         time: env.block.time + Duration::from_days(100),
//         chain_id: "cosmos-testnet-14002".to_string(),
//     };

//     let alarm_msg = ExecuteMsg::Alarm {
//         time: env.block.time,
//     };

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         mock_info("timealarms", &[]),
//         alarm_msg,
//     )
//     .unwrap();
//     assert_eq!(res.messages.len(), 3);
//     assert_eq!(
//         res.messages,
//         vec![
//             SubMsg::new(WasmMsg::Execute {
//                 contract_addr: "treasury".to_string(),
//                 msg: to_json_binary(&treasury::msg::ExecuteMsg::SendRewards {
//                     amount: Coin::<Nls>::new(44386002),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             }),
//             SubMsg::new(WasmMsg::Execute {
//                 contract_addr: "lpp".to_string(),
//                 msg: to_json_binary(&lpp::msg::ExecuteMsg::DistributeRewards {}).unwrap(),
//                 funds: coins(44386002, native_denom),
//             }),
//             SubMsg::new(WasmMsg::Execute {
//                 contract_addr: "timealarms".to_string(),
//                 msg: to_json_binary(&timealarms::msg::ExecuteMsg::AddAlarm {
//                     time: env.block.time.plus_seconds(10 * 60 * 60),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })
//         ]
//     );
// }

fn new_test_case() -> TestCase<Addr, Addr, (), (), Addr, Addr, Addr> {
    TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_treasury_with_dispatcher(Addr::unchecked("contract4"))
        .init_time_alarms()
        .init_oracle(None)
        .init_dispatcher()
        .into_generic()
}

fn query_config<Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Addr, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
) -> ConfigResponse {
    test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.dispatcher().clone(),
            &rewards_dispatcher::msg::QueryMsg::Config {},
        )
        .unwrap()
}
