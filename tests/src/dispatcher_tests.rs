use currency::{lpn::Usdc, native::Nls, Currency};
use rewards_dispatcher::{msg::ConfigResponse, ContractError};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Event},
    cw_multi_test::{AppResponse, ContractWrapper, Executor},
};

use crate::common::{
    cwcoins, lpp_wrapper::mock_lpp_query, native_cwcoin, oracle_wrapper::mock_oracle_query,
    test_case::TestCase, Native, ADDON_OPTIMAL_INTEREST_RATE, BASE_INTEREST_RATE, USER,
    UTILIZATION_OPTIMAL,
};

type Lpn = Usdc;

#[test]
fn on_alarm_zero_reward() {
    let user = Addr::unchecked(USER);

    let mut test_case: TestCase<Lpn> = TestCase::new();
    test_case
        .init(user, &mut cwcoins::<Lpn, _>(500))
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                mock_oracle_query,
            )
            .with_reply(oracle::contract::reply)
            .with_sudo(oracle::contract::sudo),
        ))
        .init_treasury_with_dispatcher(Addr::unchecked("contract4"))
        .init_dispatcher();

    let time_alarms = test_case.time_alarms().clone();

    test_case.send_funds_from_admin(time_alarms.clone(), &cwcoins::<Lpn, _>(500));

    let dispatcher = test_case.dispatcher().clone();

    assert_eq!(dispatcher.as_str(), "contract4");

    let err = test_case
        .app
        .execute_contract(
            time_alarms,
            dispatcher,
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm {},
            &cwcoins::<Lpn, _>(40),
        )
        .unwrap_err();
    let root_err = err.root_cause().downcast_ref::<ContractError>();
    assert!(matches!(root_err, Some(&ContractError::ZeroReward {})));
}

#[test]
#[ignore = "No support for swapping NLS to other currencies"]
fn on_alarm() {
    let lender = Addr::unchecked(USER);

    let mut test_case: TestCase<Lpn> = TestCase::new();
    test_case
        .init(lender.clone(), &mut cwcoins::<Lpn, _>(500))
        .init_timealarms()
        .init_oracle(None)
        .init_lpp(
            Some(
                ContractWrapper::new(
                    lpp::contract::execute,
                    lpp::contract::instantiate,
                    mock_lpp_query,
                )
                .with_sudo(lpp::contract::sudo),
            ),
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                mock_oracle_query,
            )
            .with_reply(oracle::contract::reply)
            .with_sudo(oracle::contract::sudo),
        ))
        .init_treasury()
        .init_dispatcher();

    let lpp = test_case.lpp().clone();
    let time_alarms = test_case.time_alarms().clone();
    let treasury = test_case.treasury().clone();
    let dispatcher = test_case.dispatcher().clone();

    test_case.send_funds_from_admin(time_alarms.clone(), &cwcoins::<Lpn, _>(500));

    assert_eq!(
        test_case
            .app
            .wrap()
            .query_balance(lpp.clone(), Nls::TICKER)
            .unwrap(),
        CwCoin::new(0, Nls::TICKER)
    );

    let treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(treasury.clone())
        .unwrap();

    println!("treasury_balance = {:?}", treasury_balance);

    // make a deposit to LPP from lenders address
    let _res = test_case
        .app
        .execute_contract(
            lender.clone(),
            lpp.clone(),
            &lpp::msg::ExecuteMsg::Deposit {},
            &cwcoins::<Lpn, _>(100),
        )
        .unwrap();

    // call dispatcher on alarm
    let res = test_case
        .app
        .execute_contract(
            time_alarms,
            dispatcher.clone(),
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(res.events.len(), 8, "{:?}", res.events);

    let dispatcher_exec = &res.events[0];
    assert_eq!(dispatcher_exec.ty.as_str(), "execute");
    assert_eq!(
        dispatcher_exec.attributes,
        [("_contract_addr", dispatcher.as_str())]
    );
    let treasury_exec = &res.events[1];
    assert_eq!(treasury_exec.ty.as_str(), "wasm-tr-rewards");
    assert_eq!(
        treasury_exec.attributes,
        [
            ("_contract_addr", test_case.dispatcher().to_string()),
            ("rewards-amount", String::from("11")),
            ("rewards-symbol", String::from(Nls::TICKER)),
            ("height", test_case.app.block_info().height.to_string()),
            ("at", test_case.app.block_info().time.nanos().to_string()),
            ("idx", 0.to_string()),
            ("to", test_case.lpp().to_string()),
        ]
    );
    let treasury_exec = &res.events[2];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", treasury.as_str())]
    );
    let treasury_wasm = &res.events[3];
    assert_eq!(treasury_wasm.ty.as_str(), "wasm");
    assert_eq!(
        treasury_wasm.attributes,
        [
            ("_contract_addr", treasury.as_str()),
            ("method", "try_send_rewards")
        ]
    );

    let treasury_exec = &res.events[4];
    assert_eq!(treasury_exec.ty.as_str(), "transfer");
    assert_eq!(
        treasury_exec.attributes,
        [
            ("recipient", dispatcher.as_str()),
            ("sender", treasury.as_str()),
            ("amount", &format!("11{}", Native::BANK_SYMBOL))
        ]
    );

    let treasury_exec = &res.events[5];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(treasury_exec.attributes, [("_contract_addr", &lpp)]);
    let treasury_exec = &res.events[6];
    assert_eq!(treasury_exec.ty.as_str(), "wasm");
    assert_eq!(
        treasury_exec.attributes,
        [
            ("_contract_addr", lpp.as_str()),
            ("method", "try_distribute_rewards")
        ]
    );

    assert_eq!(
        test_case
            .app
            .wrap()
            .query_balance(lpp.clone(), Native::BANK_SYMBOL)
            .unwrap(),
        native_cwcoin(11)
    );

    //query calculated reward for the lender
    let resp: lpp::msg::RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &lpp::msg::QueryMsg::Rewards { address: lender })
        .unwrap();

    println!("LPP rewards {:?}", resp);
}

#[test]
fn test_config() {
    let mut test_case = new_test_case();

    let dispatcher = test_case.dispatcher().clone();

    let resp = query_config(&test_case);
    assert_eq!(resp.cadence_hours, 10);

    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            dispatcher,
            &rewards_dispatcher::msg::SudoMsg::Config { cadence_hours: 30 },
        )
        .unwrap();
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
//                 msg: to_binary(&treasury::msg::ExecuteMsg::SendRewards {
//                     amount: Coin::<Nls>::new(44386002),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             }),
//             SubMsg::new(WasmMsg::Execute {
//                 contract_addr: "lpp".to_string(),
//                 msg: to_binary(&lpp::msg::ExecuteMsg::DistributeRewards {}).unwrap(),
//                 funds: coins(44386002, native_denom),
//             }),
//             SubMsg::new(WasmMsg::Execute {
//                 contract_addr: "timealarms".to_string(),
//                 msg: to_binary(&timealarms::msg::ExecuteMsg::AddAlarm {
//                     time: env.block.time.plus_seconds(10 * 60 * 60),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })
//         ]
//     );
// }

fn new_test_case() -> TestCase<Lpn> {
    let mut test_case: TestCase<Lpn> = TestCase::new();
    test_case
        .init_lease()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_treasury_with_dispatcher(Addr::unchecked("contract4"))
        .init_timealarms()
        .init_oracle(None)
        .init_dispatcher();

    test_case
}

fn query_config(test_case: &TestCase<Lpn>) -> ConfigResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.dispatcher().clone(),
            &rewards_dispatcher::msg::QueryMsg::Config {},
        )
        .unwrap()
}
