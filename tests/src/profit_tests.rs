use currencies::{Lpn, Lpns, Nls};
use currency::{CurrencyDef, MemberOf};
use finance::{coin::Coin, duration::Duration, zero::Zero};
use platform::bank;
use profit::{
    CadenceHours,
    msg::{ConfigResponse, ExecuteMsg, QueryMsg},
};
use sdk::{
    cosmwasm_std::{self, Addr, Event},
    testing,
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::common::{
    self, ADMIN, USER,
    protocols::Registry,
    test_case::{TestCase, builder::BlankBuilder as TestCaseBuilder},
};

fn test_case_with<Lpn>(
    cadence_hours: CadenceHours,
) -> TestCase<Addr, Addr, Addr, (), (), (), Addr, Addr>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    TestCaseBuilder::<Lpn>::new()
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(cadence_hours)
        .into_generic()
}

fn test_case<Lpn>() -> TestCase<Addr, Addr, Addr, (), (), (), Addr, Addr>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    test_case_with::<Lpn>(2)
}

#[test]
fn update_config() {
    const INITIAL_CACDENCE_HOURS: CadenceHours = 2;
    const UPDATED_CACDENCE_HOURS: CadenceHours = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CACDENCE_HOURS);

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
            testing::user(ADMIN),
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
    const INITIAL_CACDENCE_HOURS: CadenceHours = 2;
    const UPDATED_CACDENCE_HOURS: CadenceHours = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CACDENCE_HOURS);

    assert!(
        test_case
            .app
            .execute(
                testing::user(USER),
                test_case.address_book.profit().clone(),
                &ExecuteMsg::Config {
                    cadence_hours: UPDATED_CACDENCE_HOURS
                },
                &[],
            )
            .unwrap_err()
            .to_string()
            .contains("Unauthorized")
    );
}

#[test]
fn on_alarm_from_unknown() {
    let user_addr: Addr = testing::user(USER);

    let mut test_case = test_case::<Lpn>();

    test_case.send_funds_from_admin(user_addr.clone(), &[common::cwcoin_from_amount::<Lpn>(500)]);

    let settlement_balance =
        common::query_all_balances(test_case.address_book.settlement(), test_case.app.query());

    _ = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.profit().clone(),
            &ExecuteMsg::TimeAlarm {},
            &[common::cwcoin_from_amount::<Lpn>(40)],
        )
        .unwrap_err();

    // nothing is dumped to the settlement account
    assert_eq!(
        settlement_balance,
        common::query_all_balances(test_case.address_book.settlement(), test_case.app.query()),
    );
}

#[test]
fn on_alarm_zero_balance() {
    let mut test_case = test_case::<Lpn>();

    () = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
}

#[test]
fn on_alarm_dumps_every_currency_to_settlement() {
    let native_profit: Coin<Nls> = common::coin(1000);
    let lpn_profit: Coin<Lpn> = common::lpn_coin(500);

    let mut test_case = test_case::<Lpn>();

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[common::cwcoin(native_profit), common::cwcoin(lpn_profit)],
    );

    () = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    // profit is fully drained
    assert_eq!(
        bank::balance::<Nls>(test_case.address_book.profit(), test_case.app.query()).unwrap(),
        Zero::ZERO,
    );
    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), test_case.app.query()).unwrap(),
        Zero::ZERO,
    );

    // every balance lands on the settlement account, untouched by any swap
    assert_eq!(
        bank::balance::<Nls>(test_case.address_book.settlement(), test_case.app.query()).unwrap(),
        native_profit,
    );
    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.settlement(), test_case.app.query()).unwrap(),
        lpn_profit,
    );
}

#[test]
fn integration_with_time_alarms() {
    const CADENCE_HOURS: CadenceHours = 2;

    let profit_amount: Coin<Nls> = common::coin(500);

    let mut test_case = test_case_with::<Lpn>(CADENCE_HOURS);

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[common::cwcoin(profit_amount)],
    );

    let resp = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_eq!(
        cosmwasm_std::from_json::<DispatchAlarmsResponse>(resp.data.clone().unwrap()).unwrap(),
        DispatchAlarmsResponse(1)
    );

    resp.assert_event(&Event::new("wasm-time-alarm").add_attribute("delivered", "success"));

    // the whole balance is dumped, no reserve is held back
    assert_eq!(
        bank::balance::<Nls>(test_case.address_book.profit(), test_case.app.query()).unwrap(),
        Zero::ZERO,
    );
    assert_eq!(
        bank::balance::<Nls>(test_case.address_book.settlement(), test_case.app.query()).unwrap(),
        profit_amount,
    );
}
