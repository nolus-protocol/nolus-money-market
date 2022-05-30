use cosmwasm_std::{coins, Addr};
use cw_multi_test::{ContractWrapper, Executor};

use crate::tests::common::{
    mock_lpp::{mock_lpp_execute, mock_lpp_query},
    mock_oracle::mock_oracle_query,
    test_case::TestCase,
};

// pub fn setup_test_case(
//     app: &mut App,
//     init_funds: Vec<Coin>,
//     user_addr: Addr,
//     denom: &str,
// ) -> (Addr, Addr, Addr) {
//     let lease_id = app.store_code(contract_lease_mock());

//     // 1. Instantiate LPP contract
//     let (lpp_addr, _lpp_id) = MockLpp::default().instantiate(app, Uint64::new(lease_id), denom);
//     app.update_block(next_block);

//     // 2. Instantiate Treasury contract (and OWNER as admin)
//     let treasury_addr = MockTreasury::default().instantiate(app, denom);
//     app.update_block(next_block);

//     // 3. Instantiate Oracle contract (and OWNER as admin)
//     let market_oracle = instantiate_oracle(app, denom);
//     app.update_block(next_block);

//     // 3. Instantiate Dispatcher contract
//     let dispatcher_addr = MockDispatcher::default().instantiate(
//         app,
//         &lpp_addr,
//         &Addr::unchecked("time"),
//         &treasury_addr,
//         &market_oracle,
//         denom,
//     );
//     app.update_block(next_block);

//     // Bonus: set some funds on the user for future proposals
//     if !init_funds.is_empty() {
//         app.send_tokens(Addr::unchecked(ADMIN), user_addr, &init_funds)
//             .unwrap();
//     }
//     (dispatcher_addr, treasury_addr, lpp_addr)
// }

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
            &crate::msg::ExecuteMsg::Alarm {
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
            &crate::msg::ExecuteMsg::Alarm {
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
                test_case.treasury_addr.unwrap().to_string()
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
            ("sender", test_case.lpp_addr.clone().unwrap().to_string()),
            ("amount", "32UST".to_string())
        ]
    );
    let treasury_exec = &res.events[4];
    assert_eq!(treasury_exec.ty.as_str(), "execute");
    assert_eq!(
        treasury_exec.attributes,
        [("_contract_addr", &test_case.lpp_addr.unwrap())]
    );
}

// #[test]
// fn test_config() {
//     let denom = "UST";
//     let user_addr = Addr::unchecked(USER);
//     let mut test_case = TestCase::new(denom);
//     test_case.init(&user_addr, coins(500, denom));
//     test_case.init_dispatcher();

//     let resp: crate::msg::ConfigResponse = test_case
//         .app
//         .wrap()
//         .query_wasm_smart(test_case.dispatcher_addr.unwrap(), &QueryMsg::Config {})
//         .unwrap();

//     assert_eq!(10, resp.cadence_hours);
// }
