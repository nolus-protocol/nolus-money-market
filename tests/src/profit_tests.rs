use currency::{lpn::Usdc, Currency};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    zero::Zero as _,
};
use platform::bank;
use profit::msg::{ConfigResponse, ExecuteMsg, QueryMsg};
use sdk::{
    cosmwasm_std::{from_binary, Addr, Event},
    cw_multi_test::AppResponse,
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::common::{
    cwcoin,
    test_case::{
        builder::BlankBuilder as TestCaseBuilder,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
        TestCase,
    },
    Native, ADMIN, USER,
};

#[test]
fn update_config() {
    type Lpn = Usdc;

    const INITIAL_CACDENCE_HOURS: u16 = 2;
    const UPDATED_CACDENCE_HOURS: u16 = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(INITIAL_CACDENCE_HOURS)
        .into_generic();

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(cadence_hours, INITIAL_CACDENCE_HOURS);

    () = test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Config {
                cadence_hours: UPDATED_CACDENCE_HOURS,
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(cadence_hours, UPDATED_CACDENCE_HOURS);
}

#[test]
fn update_config_unauthorized() {
    type Lpn = Usdc;

    const INITIAL_CACDENCE_HOURS: u16 = 2;
    const UPDATED_CACDENCE_HOURS: u16 = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(INITIAL_CACDENCE_HOURS)
        .into_generic();

    assert!(test_case
        .app
        .execute(
            Addr::unchecked(USER),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Config {
                cadence_hours: UPDATED_CACDENCE_HOURS
            },
            &[],
        )
        .unwrap_err()
        .to_string()
        .contains("Unauthorized"));
}

#[test]
fn on_alarm_from_unknown() {
    type Lpn = Usdc;
    let user_addr: Addr = Addr::unchecked(USER);

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    test_case.send_funds_from_admin(user_addr.clone(), &[cwcoin::<Lpn, _>(500)]);

    let treasury_balance = test_case
        .app
        .query()
        .query_all_balances(test_case.address_book.treasury().clone())
        .unwrap();

    _ = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[cwcoin::<Lpn, _>(40)],
        )
        .unwrap_err();

    //assert that no transfer is made to treasury
    assert_eq!(
        treasury_balance,
        test_case
            .app
            .query()
            .query_all_balances(test_case.address_book.treasury().clone())
            .unwrap()
    );
}

#[test]
fn on_alarm_zero_balance() {
    type Lpn = Usdc;
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    test_case.send_funds_from_admin(time_oracle_addr, &[cwcoin::<Lpn, _>(500)]);

    () = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
}

#[test]
fn on_alarm_native_only_transfer() {
    type Lpn = Usdc;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    let init_balance_nls = bank::balance::<Native>(
        &test_case.address_book.treasury().clone(),
        &test_case.app.query(),
    )
    .unwrap();
    let init_balance_lpn = bank::balance::<Lpn>(
        &test_case.address_book.treasury().clone(),
        &test_case.app.query(),
    )
    .unwrap();
    let profit = Coin::<Native>::from(1000);
    let sent_profit = profit - ::profit::profit::Profit::IBC_FEE_RESERVE;

    //send tokens to the profit contract
    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[cwcoin::<Native, _>(profit)],
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), &test_case.app.query()).unwrap(),
        Coin::ZERO,
    );

    let response = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .unwrap_response();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(response.events.len(), 4, "{:?}", response.events);

    let profit_exec = &response.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.address_book.profit().as_str())]
    );

    let profit_exec = &response.events[1];
    assert_eq!(profit_exec.ty.as_str(), "wasm-tr-profit");
    assert_eq!(
        profit_exec.attributes,
        [
            ("_contract_addr", test_case.address_book.profit().as_str()),
            ("height", &test_case.app.block_info().height.to_string()),
            ("at", &test_case.app.block_info().time.nanos().to_string()),
            ("idx", "0"),
            (
                "profit-amount-amount",
                &Amount::from(sent_profit).to_string()
            ),
            ("profit-amount-symbol", Native::TICKER)
        ]
    );

    let profit_exec = &response.events[2];
    assert_eq!(profit_exec.ty.as_str(), "transfer");
    assert_eq!(
        profit_exec.attributes,
        [
            ("recipient", test_case.address_book.treasury().as_str()),
            ("sender", test_case.address_book.profit().as_str()),
            (
                "amount",
                &format!("{}{}", Amount::from(sent_profit), Native::BANK_SYMBOL)
            )
        ]
    );

    let profit_exec = &response.events[3];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.address_book.time_alarms())]
    );

    assert_eq!(
        bank::balance::<Native>(test_case.address_book.treasury(), &test_case.app.query()).unwrap(),
        init_balance_nls + sent_profit,
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), &test_case.app.query()).unwrap(),
        Coin::ZERO,
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.treasury(), &test_case.app.query()).unwrap(),
        init_balance_lpn,
    );
}

#[test]
fn on_alarm_foreign_only_transfer() {
    type Lpn = Usdc;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    let profit_lpn = Coin::<Lpn>::from(100);

    //send tokens to the profit contract
    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[cwcoin::<Lpn, _>(profit_lpn)],
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), &test_case.app.query()).unwrap(),
        profit_lpn,
    );

    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    response.expect_ibc_transfer(
        TestCase::PROFIT_ICA_CHANNEL,
        cwcoin(profit_lpn),
        test_case.address_book.profit().as_str(),
        TestCase::PROFIT_ICA_ADDR,
    );

    let response: AppResponse = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            test_case.address_book.profit().clone(),
            Addr::unchecked(TestCase::PROFIT_ICA_ADDR),
            &[cwcoin(profit_lpn)],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(
        response.events.as_slice(),
        &[Event::new("execute").add_attribute("_contract_addr", test_case.address_book.profit())]
    );
}

#[test]
fn on_alarm_native_and_foreign_transfer() {
    type Lpn = Usdc;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    let profit_nls = Coin::<Native>::from(100);
    let profit_lpn = Coin::<Lpn>::from(100);

    //send tokens to the profit contract
    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[
            cwcoin::<Native, Coin<Native>>(profit_nls),
            cwcoin::<Lpn, Coin<Lpn>>(profit_lpn),
        ],
    );

    assert_eq!(
        bank::balance::<Native>(test_case.address_book.profit(), &test_case.app.query()).unwrap(),
        profit_nls,
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), &test_case.app.query()).unwrap(),
        profit_lpn,
    );

    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    response.expect_ibc_transfer(
        TestCase::PROFIT_ICA_CHANNEL,
        cwcoin(profit_lpn),
        test_case.address_book.profit().as_str(),
        TestCase::PROFIT_ICA_ADDR,
    );

    let response: AppResponse = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            test_case.address_book.profit().clone(),
            Addr::unchecked(TestCase::PROFIT_ICA_ADDR),
            &[cwcoin(profit_lpn)],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(
        response.events.as_slice(),
        &[Event::new("execute").add_attribute("_contract_addr", test_case.address_book.profit())]
    );
}

#[test]
fn integration_with_time_alarms() {
    type Lpn = Usdc;
    const CADENCE_HOURS: u16 = 2;

    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(CADENCE_HOURS)
        .into_generic();

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[cwcoin::<Native, _>(500)],
    );

    assert!(!test_case
        .app
        .query()
        .query_balance(test_case.address_book.profit().clone(), Native::BANK_SYMBOL)
        .unwrap()
        .amount
        .is_zero());

    let resp = test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_eq!(
        from_binary(&resp.data.clone().unwrap()),
        Ok(DispatchAlarmsResponse(1))
    );

    resp.assert_event(&Event::new("wasm-time-alarm").add_attribute("delivered", "success"));

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.profit().clone(), Native::BANK_SYMBOL)
            .unwrap()
            .amount
            .u128(),
        ::profit::profit::Profit::IBC_FEE_RESERVE.into(),
    );
}
