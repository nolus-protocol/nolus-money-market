use currency::{lpn::Usdc, native::Nls};
use finance::currency::Currency;
use sdk::{
    cosmwasm_std::{coins, Addr, Coin as CwCoin},
    cw_multi_test::Executor,
};

use crate::common::{test_case::TestCase, ADMIN, USER};

#[test]
fn on_alarm_from_unknown() {
    let denom = Usdc::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_treasury().init_timealarms().init_profit(2);

    let treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(test_case.treasury_addr.clone().unwrap())
        .unwrap();

    let res = test_case.app.execute_contract(
        user_addr,
        test_case.profit_addr.as_ref().unwrap().clone(),
        &profit::msg::ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
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
#[should_panic(expected = "EmptyBalance. No profit to dispatch")]
fn on_alarm_zero_balance() {
    let denom = Usdc::SYMBOL;
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));
    test_case.init_treasury().init_timealarms().init_profit(2);

    test_case
        .app
        .execute_contract(
            test_case.timealarms.unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
            &[],
        )
        .unwrap();
}

#[test]
fn on_alarm_transfer() {
    let denom = Nls::SYMBOL;
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::new(denom);
    test_case.init(&time_oracle_addr, coins(500, denom));
    test_case.init_treasury().init_timealarms().init_profit(2);

    assert_eq!(
        CwCoin::new(2000, denom),
        test_case
            .app
            .wrap()
            .query_balance(test_case.treasury_addr.as_ref().unwrap(), denom)
            .unwrap()
    );

    //send tokens to the profit contract
    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            test_case.profit_addr.clone().unwrap(),
            &coins(100, denom),
        )
        .unwrap();

    let res = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(4, res.events.len(), "{:?}", res.events);
    let profit_exec = &res.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );
    let profit_exec = &res.events[1];

    assert_eq!(profit_exec.ty.as_str(), "wasm-tr-profit");
    assert_eq!(
        profit_exec.attributes,
        [
            (
                "_contract_addr",
                test_case.profit_addr.as_ref().unwrap().to_string()
            ),
            ("height", test_case.app.block_info().height.to_string()),
            ("at", test_case.app.block_info().time.nanos().to_string()),
            ("idx", String::from("0")),
            ("profit-amount-amount", String::from("100")),
            ("profit-amount-symbol", Nls::SYMBOL.to_string())
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
            (
                "sender",
                test_case.profit_addr.as_ref().unwrap().to_string()
            ),
            ("amount", format!("{}{}", 100, Nls::SYMBOL))
        ]
    );
    let profit_exec = &res.events[3];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [(
            "_contract_addr",
            test_case.timealarms.as_ref().unwrap().to_string()
        )]
    );

    assert_eq!(
        CwCoin::new(2100, denom),
        test_case
            .app
            .wrap()
            .query_balance(test_case.treasury_addr.unwrap(), denom)
            .unwrap()
    );
}
