use currency::lpn::Usdc;
use finance::{
    coin::{Amount, Coin},
    currency::Currency,
    duration::Duration,
    zero::Zero as _,
};
use platform::bank;
use sdk::{
    cosmwasm_ext::CustomMsg,
    cosmwasm_std::{from_binary, Addr, Event},
    cw_multi_test::Executor as _,
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::common::{cwcoin, cwcoins, test_case::TestCase, AppExt as _, Native, ADMIN, USER};

#[test]
fn on_alarm_from_unknown() {
    type Lpn = Usdc;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init(&user_addr, cwcoins::<Lpn, _>(500))
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(2);

    let treasury_balance = test_case
        .app
        .wrap()
        .query_all_balances(test_case.treasury_addr.clone().unwrap())
        .unwrap();

    let res = test_case.app.execute_contract(
        user_addr,
        test_case.profit_addr.as_ref().unwrap().clone(),
        &profit::msg::ExecuteMsg::TimeAlarm {},
        &cwcoins::<Lpn, _>(40),
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
    type Lpn = Usdc;
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init(&time_oracle_addr, cwcoins::<Lpn, _>(500))
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(2);

    test_case
        .app
        .execute_contract(
            test_case.timealarms.unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();
}

#[test]
fn on_alarm_native_only_transfer() {
    type Lpn = Usdc;

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(2);

    let init_balance_nls = bank::balance::<Native>(
        test_case.treasury_addr.as_ref().unwrap(),
        &test_case.app.wrap(),
    )
    .unwrap();
    let init_balance_lpn = bank::balance::<Lpn>(
        test_case.treasury_addr.as_ref().unwrap(),
        &test_case.app.wrap(),
    )
    .unwrap();
    let profit = Coin::<Native>::from(100);

    //send tokens to the profit contract
    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            test_case.profit_addr.clone().unwrap(),
            &cwcoins::<Native, _>(profit),
        )
        .unwrap();

    assert_eq!(
        bank::balance::<Lpn>(
            test_case.profit_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        Coin::ZERO,
    );

    let response = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(response.events.len(), 4, "{:?}", response.events);

    let profit_exec = &response.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );

    let profit_exec = &response.events[1];

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
            ("profit-amount-amount", Amount::from(profit).to_string()),
            ("profit-amount-symbol", Native::TICKER.into())
        ]
    );

    let profit_exec = &response.events[2];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [(
            "_contract_addr",
            test_case.timealarms.as_ref().unwrap().to_string()
        )]
    );

    let profit_exec = &response.events[3];
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
            (
                "amount",
                format!("{}{}", Amount::from(profit), Native::BANK_SYMBOL)
            )
        ]
    );

    assert_eq!(
        bank::balance::<Native>(
            test_case.treasury_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        init_balance_nls + profit,
    );

    assert_eq!(
        bank::balance::<Lpn>(
            test_case.profit_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        Coin::ZERO,
    );

    assert_eq!(
        bank::balance::<Lpn>(
            test_case.treasury_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        init_balance_lpn,
    );
}

#[test]
fn on_alarm_foreign_only_transfer() {
    type Lpn = Usdc;

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(2);

    let profit_lpn = Coin::<Lpn>::from(100);

    //send tokens to the profit contract
    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            test_case.profit_addr.clone().unwrap(),
            &cwcoins::<Lpn, _>(profit_lpn),
        )
        .unwrap();

    assert_eq!(
        bank::balance::<Lpn>(
            test_case.profit_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        profit_lpn,
    );

    let response = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    assert!(matches!(
        test_case
            .message_receiver
            .try_recv()
            .expect("Expected IBC transfer message!"),
        CustomMsg::IbcTransfer { .. }
    ));

    // ensure the attributes were relayed from the sub-message
    assert_eq!(
        response.events.as_slice(),
        &[Event::new("execute")
            .add_attribute("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );
}

#[test]
fn on_alarm_native_and_foreign_transfer() {
    type Lpn = Usdc;

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(2);

    let profit_nls = Coin::<Native>::from(100);
    let profit_lpn = Coin::<Lpn>::from(100);

    //send tokens to the profit contract
    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            test_case.profit_addr.clone().unwrap(),
            &[
                cwcoin::<Native, Coin<Native>>(profit_nls),
                cwcoin::<Lpn, Coin<Lpn>>(profit_lpn),
            ],
        )
        .unwrap();

    assert_eq!(
        bank::balance::<Native>(
            test_case.profit_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        profit_nls,
    );

    assert_eq!(
        bank::balance::<Lpn>(
            test_case.profit_addr.as_ref().unwrap(),
            &test_case.app.wrap(),
        )
        .unwrap(),
        profit_lpn,
    );

    let response = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            test_case.profit_addr.as_ref().unwrap().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    assert!(matches!(
        test_case
            .message_receiver
            .try_recv()
            .expect("Expected IBC transfer message!"),
        CustomMsg::IbcTransfer { .. }
    ));

    // ensure the attributes were relayed from the sub-message
    assert_eq!(
        response.events.as_slice(),
        &[Event::new("execute")
            .add_attribute("_contract_addr", test_case.profit_addr.as_ref().unwrap())]
    );
}

#[test]
fn integration_with_timealarms() {
    type Lpn = Usdc;
    const CADENCE_HOURS: u16 = 2;

    let mut test_case = TestCase::<Lpn>::new();

    test_case
        .init_treasury()
        .init_timealarms()
        .init_oracle(None)
        .init_profit(CADENCE_HOURS);

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds(
        &test_case.profit_addr.clone().unwrap(),
        cwcoins::<Native, _>(500),
    );

    assert!(!test_case
        .app
        .wrap()
        .query_balance(test_case.profit_addr.clone().unwrap(), Native::BANK_SYMBOL,)
        .unwrap()
        .amount
        .is_zero());

    let resp = test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.timealarms.unwrap(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap();
    assert_eq!(
        from_binary(&resp.data.clone().unwrap()),
        Ok(DispatchAlarmsResponse(1))
    );
    resp.assert_event(&Event::new("wasm-time-alarm").add_attribute("delivered", "success"));

    assert!(test_case
        .app
        .wrap()
        .query_balance(test_case.profit_addr.clone().unwrap(), Native::BANK_SYMBOL,)
        .unwrap()
        .amount
        .is_zero());
}
