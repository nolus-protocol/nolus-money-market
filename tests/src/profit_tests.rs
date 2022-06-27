use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::Executor;

use crate::common::{test_case::TestCase, ADMIN, USER};

#[test]
fn on_alarm_from_unknown() {
    let denom = "UST";
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_treasury().init_oracle(None).init_profit(2);

    let treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(test_case.treasury_addr.clone().unwrap())
        .unwrap();

    let res = test_case.app.execute_contract(
        user_addr,
        test_case.profit_addr.as_ref().unwrap().clone(),
        &profit::msg::ExecuteMsg::Alarm {
            time: test_case.app.block_info().time,
        },
        &coins(40, denom),
    );
    assert!(res.is_err());

    //assert that no transfer is made to treasury
    assert_eq!(
        treasury_balance,
        test_case
            .app
            .wrap()
            .query_all_balances(test_case.treasury_addr.unwrap())
            .unwrap()
    );
}

#[test]
fn on_alarm_zero_balance() {
    let denom = "UST";
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));
    test_case.init_treasury().init_oracle(None).init_profit(2);

    let initial_treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(test_case.treasury_addr.clone().unwrap())
        .unwrap();

    let res = test_case
        .app
        .execute_contract(
            test_case.oracle.unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::Alarm {
                time: test_case.app.block_info().time,
            },
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(2, res.events.len(), "{:?}", res.events);
    let profit_exec = &res.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );
    let profit_exec = &res.events[1];
    assert_eq!(profit_exec.ty.as_str(), "wasm");
    assert_eq!(
        profit_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.profit_addr.as_ref().unwrap().to_string()
            ),
            ("method", "try_transfer".to_string()),
            ("result", "no profit to dispatch".to_string())
        ]
    );

    // assert no change in treasury balance
    assert_eq!(
        initial_treasury_balance,
        test_case
            .app
            .wrap()
            .query_all_balances(test_case.treasury_addr.unwrap())
            .unwrap()
    );
}

#[test]
fn on_alarm_transfer() {
    let denom = "UST";
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));
    test_case.init_treasury().init_oracle(None).init_profit(2);

    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            test_case.profit_addr.clone().unwrap(),
            &coins(100, "UST"),
        )
        .unwrap();

    let res = test_case
        .app
        .execute_contract(
            test_case.oracle.unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::Alarm {
                time: test_case.app.block_info().time,
            },
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(3, res.events.len(), "{:?}", res.events);
    let profit_exec = &res.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );
    let profit_exec = &res.events[1];
    assert_eq!(profit_exec.ty.as_str(), "wasm");
    assert_eq!(
        profit_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.profit_addr.as_ref().unwrap().to_string()
            ),
            ("method", "try_transfer".to_string())
        ]
    );
    let profit_exec = &res.events[2];
    assert_eq!(profit_exec.ty.as_str(), "transfer");
    assert_eq!(
        profit_exec.attributes,
        [
            (
                "recipient",
                test_case.treasury_addr.as_ref().unwrap().to_string()
            ),
            ("sender", test_case.profit_addr.unwrap().to_string()),
            ("amount", "100UST".to_string())
        ]
    );

    assert_eq!(
        Coin::new(1100, denom),
        test_case
            .app
            .wrap()
            .query_balance(test_case.treasury_addr.unwrap(), denom)
            .unwrap()
    );
}
