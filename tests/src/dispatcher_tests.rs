use currency::{lpn::Usdc, native::Nls};
use finance::currency::Currency;
use platform::access_control::Unauthorized;
use rewards_dispatcher::ContractError;
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::{ContractWrapper, Executor},
};

use crate::common::{
    cwcoins, lpp_wrapper::mock_lpp_query, native_cwcoin, oracle_wrapper::mock_oracle_query,
    test_case::TestCase, Native, ADMIN, USER,
};

#[test]
fn on_alarm_zero_reward() {
    type Lpn = Usdc;

    let user = Addr::unchecked(USER);
    let mut test_case = TestCase::<Usdc>::new();
    test_case.init(&user, cwcoins::<Lpn, _>(500));

    test_case
        .init_lpp(None)
        .init_timealarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                mock_oracle_query,
            )
            .with_reply(oracle::contract::reply),
        ))
        .init_treasury()
        .init_dispatcher();

    test_case.send_funds(
        &test_case.timealarms.clone().unwrap(),
        cwcoins::<Lpn, _>(500),
    );

    let err = test_case
        .app
        .execute_contract(
            test_case.timealarms.unwrap(),
            test_case.dispatcher_addr.as_ref().unwrap().clone(),
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
            &cwcoins::<Lpn, _>(40),
        )
        .unwrap_err();
    let root_err = err.root_cause().downcast_ref::<ContractError>();
    assert!(matches!(root_err, Some(&ContractError::ZeroReward {})));
}

#[test]
#[ignore = "No support for swapping NLS to other currencies"]
fn on_alarm() {
    type Lpn = Usdc;

    let lender = Addr::unchecked(USER);

    let mut test_case = TestCase::<Usdc>::new();
    test_case
        .init(&lender, cwcoins::<Lpn, _>(500))
        .init_timealarms()
        .init_oracle(None);

    test_case
        .init_lpp(Some(ContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            mock_lpp_query,
        )))
        .init_timealarms()
        .init_oracle(Some(
            ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                mock_oracle_query,
            )
            .with_reply(oracle::contract::reply),
        ))
        .init_treasury()
        .init_dispatcher();
    test_case.send_funds(
        &test_case.timealarms.clone().unwrap(),
        cwcoins::<Lpn, _>(500),
    );

    assert_eq!(
        CwCoin::new(0, Nls::TICKER),
        test_case
            .app
            .wrap()
            .query_balance(test_case.lpp_addr.clone().unwrap(), Nls::TICKER)
            .unwrap()
    );

    let treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(test_case.treasury_addr.clone().unwrap())
        .unwrap();

    println!("treasury_balance = {:?}", treasury_balance);

    // make a deposit to LPP from lenders address
    let _res = test_case
        .app
        .execute_contract(
            lender.clone(),
            test_case.lpp_addr.clone().unwrap(),
            &lpp::msg::ExecuteMsg::Deposit {},
            &cwcoins::<Lpn, _>(100),
        )
        .unwrap();

    // call dispatcher on alarm
    let res = test_case
        .app
        .execute_contract(
            test_case.timealarms.unwrap(),
            test_case.dispatcher_addr.clone().unwrap(),
            &rewards_dispatcher::msg::ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(8, res.events.len(), "{:?}", res.events);

    let dispatcher_exec = &res.events[0];
    assert_eq!(dispatcher_exec.ty.as_str(), "execute");
    assert_eq!(
        dispatcher_exec.attributes,
        [("_contract_addr", test_case.dispatcher_addr.clone().unwrap())]
    );
    let treasury_exec = &res.events[1];
    assert_eq!(treasury_exec.ty.as_str(), "wasm-tr-rewards");
    assert_eq!(
        treasury_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.dispatcher_addr.clone().unwrap().to_string()
            ),
            ("rewards-amount", String::from("11")),
            ("rewards-symbol", String::from(Nls::TICKER)),
            ("height", test_case.app.block_info().height.to_string()),
            ("at", test_case.app.block_info().time.nanos().to_string()),
            ("idx", 0.to_string()),
            ("to", test_case.lpp_addr.as_ref().unwrap().to_string()),
        ]
    );
    let treasury_exec = &res.events[2];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", &test_case.treasury_addr.clone().unwrap())]
    );
    let treasury_wasm = &res.events[3];
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

    let treasury_exec = &res.events[4];
    assert_eq!(treasury_exec.ty.as_str(), "transfer");
    assert_eq!(
        treasury_exec.attributes,
        [
            (
                "recipient",
                &test_case.dispatcher_addr.clone().unwrap().to_string()
            ),
            (
                "sender",
                &test_case.treasury_addr.clone().unwrap().to_string()
            ),
            ("amount", &format!("11{}", Native::BANK_SYMBOL))
        ]
    );

    let treasury_exec = &res.events[5];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", &test_case.lpp_addr.clone().unwrap())]
    );
    let treasury_exec = &res.events[6];
    assert_eq!(treasury_exec.ty.as_str(), "wasm");
    assert_eq!(
        treasury_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.lpp_addr.clone().unwrap().to_string()
            ),
            ("method", "try_distribute_rewards".to_string())
        ]
    );

    assert_eq!(
        native_cwcoin(11),
        test_case
            .app
            .wrap()
            .query_balance(test_case.lpp_addr.clone().unwrap(), Native::BANK_SYMBOL)
            .unwrap()
    );

    //query calculated reward for the lender
    let resp: lpp::msg::RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.lpp_addr.clone().unwrap(),
            &lpp::msg::QueryMsg::Rewards { address: lender },
        )
        .unwrap();

    println!("LPP rewards {:?}", resp);
}

#[test]
fn test_config_unauthorized() {
    type Lpn = Usdc;
    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Usdc>::new();
    test_case
        .init(&user_addr, cwcoins::<Lpn, _>(500))
        .init_lpp(None)
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
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

    assert_eq!(
        test_case
            .app
            .execute_contract(
                user_addr,
                test_case.dispatcher_addr.clone().unwrap(),
                &rewards_dispatcher::msg::ExecuteMsg::Config { cadence_hours: 30 },
                &cwcoins::<Lpn, _>(40),
            )
            .unwrap_err()
            .downcast_ref::<ContractError>()
            .unwrap(),
        &ContractError::Unauthorized(Unauthorized),
    );
}

#[test]
fn test_config() {
    type Lpn = Usdc;
    let user_addr = Addr::unchecked(ADMIN);
    let mut test_case = TestCase::<Usdc>::new();
    test_case
        .init(&user_addr, cwcoins::<Lpn, _>(500))
        .init_lpp(None)
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
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
            &cwcoins::<Lpn, _>(40),
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
