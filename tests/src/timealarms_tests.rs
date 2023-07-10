use std::array::from_fn;

use currency::{lpn::Usdc, native::Nls, Currency};
use finance::{coin::Coin, duration::Duration};
use platform::tests;
use sdk::{
    cosmwasm_std::{coin, Addr, Attribute, Event, Timestamp},
    cw_multi_test::AppResponse,
};
use timealarms::msg::{AlarmsCount, DispatchAlarmsResponse};

use crate::common::{
    cwcoin,
    test_case::{builder::BlankBuilder as TestCaseBuilder, TestCase},
    ADMIN,
};

use self::mock_lease::*;

/// The mock for lease SC. It mimics the scheme for time notification.
/// If GATE, it returns Ok on notifications, returns Err otherwise.
mod mock_lease {
    use serde::{Deserialize, Serialize};

    use finance::duration::Duration;
    use platform::{message::Response as PlatformResponse, response};
    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{
            to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, StdError, StdResult,
        },
        cw_storage_plus::Item,
        schemars::{self, JsonSchema},
        testing::{CwContract, CwContractWrapper},
    };
    use timealarms::stub::TimeAlarmsRef;

    use crate::common::{test_case::app::App, ADMIN};

    const GATE: Item<'static, bool> = Item::new("alarm gate");
    const TIMEALARMS_ADDR: Item<'static, Addr> = Item::new("ta_addr");

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub struct MockInstantiateMsg {
        time_alarms_contract: Addr,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum MockExecuteMsg {
        // mimic the scheme
        TimeAlarm {},
        // setup GATE
        Gate(bool),
    }

    fn instantiate(
        deps: DepsMut<'_>,
        _: Env,
        _: MessageInfo,
        msg: MockInstantiateMsg,
    ) -> StdResult<Response> {
        GATE.save(deps.storage, &true)?;
        TIMEALARMS_ADDR.save(deps.storage, &msg.time_alarms_contract)?;
        Ok(Response::new().add_attribute("method", "instantiate"))
    }

    fn execute(_: DepsMut<'_>, _: Env, _: MessageInfo, msg: MockExecuteMsg) -> StdResult<Response> {
        match msg {
            MockExecuteMsg::TimeAlarm {} => Ok(Response::new()),
            MockExecuteMsg::Gate(_) => unreachable!(),
        }
    }

    fn execute_may_fail(
        deps: DepsMut<'_>,
        env: Env,
        _: MessageInfo,
        msg: MockExecuteMsg,
    ) -> StdResult<Response> {
        match msg {
            MockExecuteMsg::TimeAlarm {} => {
                let gate = GATE.load(deps.storage).expect("storage problem");

                if gate {
                    Ok(Response::new()
                        .add_attribute("lease_reply", env.block.time.to_string())
                        .set_data(to_binary(&env.contract.address)?))
                } else {
                    Err(StdError::generic_err("closed gate"))
                }
            }
            MockExecuteMsg::Gate(gate) => {
                GATE.update(deps.storage, |_| -> StdResult<bool> { Ok(gate) })?;

                Ok(Response::new().add_attribute("method", "set_gate"))
            }
        }
    }

    fn execute_reschedule_alarm(
        deps: DepsMut<'_>,
        env: Env,
        _: MessageInfo,
        msg: MockExecuteMsg,
    ) -> StdResult<Response> {
        match msg {
            MockExecuteMsg::TimeAlarm {} => {
                let timealarms = TIMEALARMS_ADDR
                    .load(deps.storage)
                    .expect("test setup error");
                let batch = TimeAlarmsRef::unchecked(timealarms)
                    .setup_alarm(env.block.time + Duration::from_secs(5))
                    .unwrap();

                Ok(response::response_only_messages(
                    PlatformResponse::messages_only(batch),
                ))
            }
            MockExecuteMsg::Gate(_gate) => {
                unimplemented!()
            }
        }
    }

    fn query(_: Deps<'_>, _: Env, _msg: MockExecuteMsg) -> StdResult<Binary> {
        Err(StdError::generic_err("not implemented"))
    }

    fn contract_no_reschedule_endpoints() -> Box<CwContract> {
        let contract = CwContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    fn contract_may_fail_endpoints() -> Box<CwContract> {
        let contract = CwContractWrapper::new(execute_may_fail, instantiate, query);
        Box::new(contract)
    }

    fn contract_reschedule_endpoints() -> Box<CwContract> {
        let contract = CwContractWrapper::new(execute_reschedule_alarm, instantiate, query);
        Box::new(contract)
    }

    pub(crate) fn instantiate_no_reschedule_contract(app: &mut App) -> Addr {
        proper_instantiate(
            app,
            contract_no_reschedule_endpoints(),
            Addr::unchecked("DEADCODE"),
        )
    }

    pub(crate) fn instantiate_may_fail_contract(app: &mut App) -> Addr {
        proper_instantiate(
            app,
            contract_may_fail_endpoints(),
            Addr::unchecked("DEADCODE"),
        )
    }

    pub(crate) fn instantiate_reschedule_contract(
        app: &mut App,
        timealarms_contract: Addr,
    ) -> Addr {
        proper_instantiate(app, contract_reschedule_endpoints(), timealarms_contract)
    }

    fn proper_instantiate(
        app: &mut App,
        endpoints: Box<CwContract>,
        timealarms_contract: Addr,
    ) -> Addr {
        let cw_template_id = app.store_code(endpoints);
        app.instantiate(
            cw_template_id,
            Addr::unchecked(ADMIN),
            &MockInstantiateMsg {
                time_alarms_contract: timealarms_contract,
            },
            &[],
            "test",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}

type Lpn = Usdc;

#[test]
fn test_lease_serde() {
    use lease::api::ExecuteMsg::TimeAlarm as LeaseTimeAlarm;
    use timealarms::msg::ExecuteAlarmMsg::TimeAlarm;

    let LeaseTimeAlarm {} = serde_json_wasm::from_slice(&serde_json_wasm::to_vec(&TimeAlarm {}).unwrap()).unwrap() else {
        unreachable!()
    };

    let TimeAlarm {} =
        serde_json_wasm::from_slice(&serde_json_wasm::to_vec(&LeaseTimeAlarm {}).unwrap()).unwrap();
}

fn test_case() -> TestCase<(), (), (), (), (), (), Addr> {
    let mut test_case: TestCase<_, _, _, _, _, _, _> =
        TestCaseBuilder::<Lpn>::with_reserve(&[coin(
            10_000_000_000_000_000_000_000_000_000,
            Lpn::BANK_SYMBOL,
        )])
        .init_time_alarms()
        .into_generic();

    test_case
        .app
        .update_block(|bl| bl.time = Timestamp::from_nanos(0));

    test_case
}

fn add_alarm<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    recv: &Addr,
    time_secs: u64,
) {
    let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
        time: Timestamp::from_seconds(time_secs),
    };
    () = test_case
        .app
        .execute(
            recv.clone(),
            test_case.address_book.time_alarms().clone(),
            &alarm_msg,
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
}

fn dispatch<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    max_count: u32,
) -> AppResponse {
    let dispatch_msg = timealarms::msg::ExecuteMsg::DispatchAlarms { max_count };
    test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &dispatch_msg,
            &[],
        )
        .unwrap()
        .unwrap_response()
}

#[test]
fn fired_alarms_are_removed() {
    let mut test_case = test_case();
    let lease1 = instantiate_may_fail_contract(&mut test_case.app);
    let lease2 = instantiate_may_fail_contract(&mut test_case.app);

    add_alarm(&mut test_case, &lease1, 1);
    //overwritten
    add_alarm(&mut test_case, &lease1, 2);
    add_alarm(&mut test_case, &lease2, 3);

    // advance by 5 seconds
    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(2));

    // try to resend same alarms
    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(0));
}

#[test]
fn no_reschedule_alarm() {
    let mut test_case = test_case();
    let lease1 = instantiate_no_reschedule_contract(&mut test_case.app);

    add_alarm(&mut test_case, &lease1, 1);

    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));

    test_case.app.time_shift(Duration::from_secs(
        5 + /* One second added because of time alarms contract's granularity */ 1,
    ));

    // try to resend the newly scheduled alarms
    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(0));
}

#[test]
fn reschedule_alarm() {
    let mut test_case = test_case();

    let lease1 = instantiate_reschedule_contract(
        &mut test_case.app,
        test_case.address_book.time_alarms().clone(),
    );

    add_alarm(&mut test_case, &lease1, 1);

    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));

    test_case.app.time_shift(Duration::from_secs(
        5 + /* One second added because of time alarms contract's granularity */ 1,
    ));

    // try to resend the newly scheduled alarms
    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));
}

#[test]
fn reschedule_failed_alarm() {
    let mut test_case = test_case();

    let lease1: Addr = instantiate_may_fail_contract(&mut test_case.app);

    () = test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            lease1.clone(),
            &MockExecuteMsg::Gate(false),
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    add_alarm(&mut test_case, &lease1, 1);

    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);
    assert!(any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));

    // try to resend the newly scheduled alarms
    let resp = dispatch(&mut test_case, 100);
    assert!(any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));
}

#[test]
fn reschedule_failing_alarms_mix() {
    let mut test_case = test_case();

    let leases: [Addr; 8] = from_fn(|index| {
        let addr: Addr = instantiate_may_fail_contract(&mut test_case.app);

        () = test_case
            .app
            .execute(
                Addr::unchecked(ADMIN),
                addr.clone(),
                &MockExecuteMsg::Gate((index % 2) == 0),
                &[],
            )
            .unwrap()
            .ignore_response()
            .unwrap_response();

        add_alarm(&mut test_case, &addr, 1);

        addr
    });

    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);

    for (index, event) in resp
        .events
        .into_iter()
        .filter(|event| {
            event
                .attributes
                .iter()
                .any(|attribute| attribute.key == "delivered")
        })
        .enumerate()
    {
        // Only leases with odd indexes fail.
        let fail = (index % 2) != 0;

        assert_eq!(
            event
                .attributes
                .iter()
                .find(|attribute| attribute.key == "delivered")
                .unwrap()
                .value,
            if fail { "error" } else { "success" }
        );

        if fail {
            assert!(event
                .attributes
                .iter()
                .find(|attribute| attribute.key == "details")
                .unwrap()
                .value
                .contains(leases[index].as_str()));
        }
    }

    // try to resend the failed alarms
    let resp = dispatch(&mut test_case, 100);
    assert_eq!(sent_alarms(&resp), Some(leases.len() as u32 / 2));

    for (index, event) in resp
        .events
        .into_iter()
        .filter(|event| {
            event
                .attributes
                .iter()
                .any(|attribute| attribute.key == "delivered")
        })
        .enumerate()
    {
        assert_eq!(
            event
                .attributes
                .iter()
                .find(|attribute| attribute.key == "delivered")
                .unwrap()
                .value,
            "error"
        );

        // Only leases with odd indexes fail.
        assert!(event
            .attributes
            .iter()
            .find(|attribute| attribute.key == "details")
            .unwrap()
            .value
            .contains(leases[(index * 2) + 1].as_str()));
    }
}

#[test]
fn test_time_notify() {
    let mut test_case = test_case();

    // instantiate lease, add alarms
    let lease1 = instantiate_may_fail_contract(&mut test_case.app);
    let lease2 = instantiate_may_fail_contract(&mut test_case.app);
    let lease3 = instantiate_may_fail_contract(&mut test_case.app);
    let lease4 = instantiate_may_fail_contract(&mut test_case.app);

    add_alarm(&mut test_case, &lease1, 1);
    add_alarm(&mut test_case, &lease2, 2);

    add_alarm(&mut test_case, &lease3, 6);
    add_alarm(&mut test_case, &lease4, 7);

    // advance by 5 seconds
    test_case.app.time_shift(Duration::from_secs(5));

    let resp = dispatch(&mut test_case, 100);

    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(2));

    let resp = dispatch(&mut test_case, 100);
    assert_eq!(sent_alarms(&resp), Some(0));

    test_case.app.time_shift(Duration::from_secs(5));

    // close the GATE, lease return error on notification
    let close_gate = mock_lease::MockExecuteMsg::Gate(false);
    () = test_case
        .app
        .execute(Addr::unchecked(ADMIN), lease3.clone(), &close_gate, &[])
        .unwrap()
        .ignore_response()
        .unwrap_response();
    let resp = dispatch(&mut test_case, 100);
    dbg!(&resp);
    assert!(any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(2));
    resp.assert_event(&Event::new("wasm-timealarm").add_attribute("receiver", lease3.clone()));
    resp.assert_event(&Event::new("wasm-timealarm").add_attribute("receiver", lease4.clone()));

    // open the GATE, check for remaining alarm
    let open_gate = mock_lease::MockExecuteMsg::Gate(true);
    () = test_case
        .app
        .execute(Addr::unchecked(ADMIN), lease3.clone(), &open_gate, &[])
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));
    resp.assert_event(&Event::new("wasm-timealarm").add_attribute("receiver", lease3.clone()));

    // check if something is left
    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(0));
}

#[test]
fn test_profit_alarms() {
    let mut test_case: TestCase<_, _, _, _, _, _, _> = TestCaseBuilder::<Lpn>::with_reserve(&[
        cwcoin(Coin::<Lpn>::new(1_000_000)),
        cwcoin(Coin::<Nls>::new(1_000_000)),
    ])
    .init_time_alarms()
    .init_oracle(None)
    .init_treasury_without_dispatcher()
    .init_profit(1)
    .into_generic();

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[cwcoin(Coin::<Nls>::new(100_000))],
    );

    test_case.app.time_shift(Duration::from_hours(10));

    let dispatch_msg = timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 1 };

    let resp = test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &dispatch_msg,
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_eq!(
        resp.events.last().unwrap().attributes.last().unwrap(),
        Attribute::new("delivered", "success")
    );
}

fn sent_alarms(resp: &AppResponse) -> Option<AlarmsCount> {
    tests::parse_resp::<DispatchAlarmsResponse>(&resp.data).map(|resp| resp.0)
}

fn any_error(resp: &AppResponse) -> bool {
    tests::any_error(&resp.events)
}
