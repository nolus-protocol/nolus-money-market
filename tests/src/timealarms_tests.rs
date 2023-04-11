use currency::{lpn::Usdc, native::Nls};
use finance::{coin::Coin, currency::Currency, duration::Duration};
use sdk::{
    cosmwasm_std::{coin, from_binary, Addr, Attribute, Timestamp},
    cw_multi_test::{AppResponse, Executor},
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::{
    common::{cwcoin, test_case::TestCase, AppExt, ADMIN},
    timealarms_tests::mock_lease::proper_instantiate,
};

/// The mock for lease SC. It mimics the scheme for time notification.
/// If GATE, it returns Ok on notifications, returns Err otherwise.
mod mock_lease {
    use serde::{Deserialize, Serialize};

    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{
            to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, StdError, StdResult,
        },
        cw_storage_plus::Item,
        schemars::{self, JsonSchema},
        testing::{Contract, ContractWrapper, Executor},
    };

    use crate::common::{MockApp, ADMIN};

    const GATE: Item<'static, bool> = Item::new("alarm gate");

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum MockExecuteMsg {
        // mimic the scheme
        TimeAlarm {},
        // setup GATE
        Gate(bool),
    }

    fn instantiate(deps: DepsMut<'_>, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
        GATE.save(deps.storage, &true)?;
        Ok(Response::new().add_attribute("method", "instantiate"))
    }

    fn execute(
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

    fn query(_: Deps<'_>, _: Env, _msg: MockExecuteMsg) -> StdResult<Binary> {
        Err(StdError::generic_err("not implemented"))
    }

    fn contract_template() -> Box<Contract> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    pub fn proper_instantiate(app: &mut MockApp) -> Addr {
        let cw_template_id = app.store_code(contract_template());
        app.instantiate_contract(
            cw_template_id,
            Addr::unchecked(ADMIN),
            &Empty {},
            &[],
            "test",
            None,
        )
        .unwrap()
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

fn test_case() -> TestCase<Lpn> {
    let mut test_case = TestCase::<Lpn>::with_reserve(
        None,
        &[coin(
            10_000_000_000_000_000_000_000_000_000,
            Lpn::BANK_SYMBOL,
        )],
    );
    test_case.init(
        &Addr::unchecked(ADMIN),
        vec![coin(1_000_000_000_000_000_000_000_000, Lpn::BANK_SYMBOL)],
    );

    test_case.init_timealarms();

    test_case
        .app
        .update_block(|bl| bl.time = Timestamp::from_nanos(0));

    test_case
}

fn add_alarm(test_case: &mut TestCase<Lpn>, recv: &Addr, time_secs: u64) {
    let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
        time: Timestamp::from_seconds(time_secs),
    };
    let timealarms = test_case.timealarms.clone().unwrap();
    test_case
        .app
        .execute_contract(recv.clone(), timealarms, &alarm_msg, &[])
        .unwrap();
}

fn dispatch(test_case: &mut TestCase<Lpn>, max_count: u32) -> AppResponse {
    let dispatch_msg = timealarms::msg::ExecuteMsg::DispatchAlarms { max_count };
    test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.timealarms.clone().unwrap(),
            &dispatch_msg,
            &[],
        )
        .unwrap()
}

fn any_error(resp: &AppResponse) -> bool {
    let maybe_attr = resp
        .events
        .iter()
        .flat_map(|ev| &ev.attributes)
        .find(|atr| atr.key == "delivered");

    matches!(maybe_attr.map(|attr| attr.value.as_str()), Some("error"))
}

fn sent_alarms(resp: &AppResponse) -> Option<u32> {
    resp.data
        .as_ref()
        .map(|data| from_binary::<DispatchAlarmsResponse>(data).unwrap().0)
}

#[test]
fn fired_alarms_are_removed() {
    let mut test_case = test_case();
    let lease1 = proper_instantiate(&mut test_case.app);
    let lease2 = proper_instantiate(&mut test_case.app);

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
fn test_time_notify() {
    let mut test_case = test_case();

    // instantiate lease, add alarms
    let lease1 = proper_instantiate(&mut test_case.app);
    let lease2 = proper_instantiate(&mut test_case.app);
    let lease3 = proper_instantiate(&mut test_case.app);
    let lease4 = proper_instantiate(&mut test_case.app);

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
    test_case
        .app
        .execute_contract(Addr::unchecked(ADMIN), lease3.clone(), &close_gate, &[])
        .unwrap();
    let resp = dispatch(&mut test_case, 100);
    dbg!(&resp);
    assert!(any_error(&resp));

    // open the GATE, check for remaining alarm
    let open_gate = mock_lease::MockExecuteMsg::Gate(true);
    test_case
        .app
        .execute_contract(Addr::unchecked(ADMIN), lease3, &open_gate, &[])
        .unwrap();

    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(1));

    // check if something is left
    let resp = dispatch(&mut test_case, 100);
    assert!(!any_error(&resp));
    assert_eq!(sent_alarms(&resp), Some(0));
}

#[test]
fn test_profit_alarms() {
    let admin = Addr::unchecked(ADMIN);
    let mut test_case = TestCase::<Lpn>::with_reserve(
        None,
        &[
            cwcoin(Coin::<Lpn>::new(1_000_000)),
            cwcoin(Coin::<Nls>::new(1_000_000)),
        ],
    );
    test_case.init(
        &admin,
        vec![
            cwcoin(Coin::<Lpn>::new(100_000)),
            cwcoin(Coin::<Nls>::new(100_000)),
        ],
    );
    test_case.init_timealarms();
    test_case.init_treasury();
    test_case.init_profit(1);

    test_case
        .app
        .send_tokens(
            admin.clone(),
            test_case.profit_addr.clone().unwrap(),
            &[cwcoin(Coin::<Nls>::new(100_000))],
        )
        .unwrap();

    test_case.app.time_shift(Duration::from_hours(10));

    let dispatch_msg = timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 1 };

    let resp = test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            test_case.timealarms.clone().unwrap(),
            &dispatch_msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        resp.events.last().unwrap().attributes.last().unwrap(),
        Attribute::new("delivered", "success")
    );
}
