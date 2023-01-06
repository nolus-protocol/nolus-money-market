use currency::{lpn::Usdc, native::Nls};
use finance::{coin::Coin, currency::Currency, duration::Duration};
use sdk::{
    cosmwasm_std::{coin, Addr, Attribute, Timestamp},
    cw_multi_test::Executor,
};

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
            Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, StdError, StdResult, Timestamp,
        },
        cw_storage_plus::Item,
        schemars::{self, JsonSchema},
        testing::{Contract, ContractWrapper, Executor},
    };

    use crate::common::{MockApp, ADMIN};

    const GATE: Item<bool> = Item::new("alarm gate");

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum MockExecuteMsg {
        // mimic the scheme
        TimeAlarm(Timestamp),
        // setup GATE
        Gate(bool),
    }

    fn instantiate(deps: DepsMut, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
        GATE.save(deps.storage, &true)?;
        Ok(Response::new().add_attribute("method", "instantiate"))
    }

    fn execute(deps: DepsMut, _: Env, _: MessageInfo, msg: MockExecuteMsg) -> StdResult<Response> {
        match msg {
            MockExecuteMsg::TimeAlarm(time) => {
                let gate = GATE.load(deps.storage).expect("storage problem");
                if gate {
                    Ok(Response::new().add_attribute("lease_reply", time.to_string()))
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

    fn query(_: Deps, _: Env, _msg: MockExecuteMsg) -> StdResult<Binary> {
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
fn test_time_notify() {
    let mut test_case = TestCase::<Lpn>::with_reserve(&[coin(
        10_000_000_000_000_000_000_000_000_000,
        Lpn::BANK_SYMBOL,
    )]);
    test_case.init(
        &Addr::unchecked(ADMIN),
        vec![coin(1_000_000_000_000_000_000_000_000, Lpn::BANK_SYMBOL)],
    );

    test_case.init_timealarms();

    let timealarms = test_case.timealarms.clone().unwrap();

    let dispatch_msg = timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 100 };

    test_case
        .app
        .update_block(|bl| bl.time = Timestamp::from_nanos(0));

    // instantiate lease, add alarms
    let lease = proper_instantiate(&mut test_case.app);

    let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
        time: Timestamp::from_seconds(1),
    };
    test_case
        .app
        .execute_contract(lease.clone(), timealarms.clone(), &alarm_msg, &[])
        .unwrap();
    let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
        time: Timestamp::from_seconds(6),
    };
    test_case
        .app
        .execute_contract(lease.clone(), timealarms.clone(), &alarm_msg, &[])
        .unwrap();

    // advance by 5 seconds
    test_case.app.time_shift(Duration::from_secs(5));

    // trigger notification, the GATE is open, events are stacked for the whole chain of contracts calls
    let resp = test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            timealarms.clone(),
            &dispatch_msg,
            &[],
        )
        .unwrap();
    let attr = resp
        .events
        .iter()
        .flat_map(|ev| &ev.attributes)
        .find(|atr| atr.key == "lease_reply")
        .unwrap();
    assert_eq!(attr.value, test_case.app.block_info().time.to_string());

    test_case.app.time_shift(Duration::from_secs(5));

    // close the GATE, lease return error on notification
    let close_gate = mock_lease::MockExecuteMsg::Gate(false);
    test_case
        .app
        .execute_contract(Addr::unchecked(ADMIN), lease.clone(), &close_gate, &[])
        .unwrap();
    let resp = test_case
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            timealarms.clone(),
            &dispatch_msg,
            &[],
        )
        .unwrap();
    let attr = resp
        .events
        .iter()
        .flat_map(|ev| &ev.attributes)
        .find(|atr| atr.key == "alarm")
        .unwrap();
    assert_eq!(attr.value, "error");

    // open the GATE, check for remaining alarm
    let open_gate = mock_lease::MockExecuteMsg::Gate(true);
    test_case
        .app
        .execute_contract(Addr::unchecked(ADMIN), lease, &open_gate, &[])
        .unwrap();
    let resp = test_case
        .app
        .execute_contract(Addr::unchecked(ADMIN), timealarms, &dispatch_msg, &[])
        .unwrap();
    let attr = resp
        .events
        .iter()
        .flat_map(|ev| &ev.attributes)
        .find(|atr| atr.key == "lease_reply")
        .unwrap();
    assert_eq!(attr.value, test_case.app.block_info().time.to_string());
}

#[test]
fn test_profit_alarms() {
    let admin = Addr::unchecked(ADMIN);
    let mut test_case = TestCase::<Lpn>::with_reserve(&[
        cwcoin(Coin::<Lpn>::new(1_000_000)),
        cwcoin(Coin::<Nls>::new(1_000_000)),
    ]);
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
        Attribute::new("alarm", "success")
    );
}
