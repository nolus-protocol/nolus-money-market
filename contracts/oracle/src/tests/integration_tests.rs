#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        to_binary, Addr, Binary, Coin, CosmosMsg, Deps, Empty, Env, StdResult, Uint128, WasmMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use crate::{msg::ExecuteMsg, tests::common::dummy_default_instantiate_msg};

    /// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
    /// for working with this.
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct CwTemplateContract(pub Addr);

    #[derive(Serialize, Clone, Debug, PartialEq)]
    struct MockResponse {}
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct QueryMsg {}

    impl CwTemplateContract {
        pub fn addr(&self) -> Addr {
            self.0.clone()
        }

        pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
            let msg = to_binary(&msg.into())?;
            Ok(WasmMsg::Execute {
                contract_addr: self.addr().into(),
                msg,
                funds: vec![],
            }
            .into())
        }
    }

    fn mock_query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
        to_binary(&MockResponse {})
    }
    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }
    pub fn timealarms_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            timealarms::contract::execute,
            timealarms::contract::instantiate,
            mock_query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }
    const USER: &str = "user";
    const ADMIN: &str = "admin";
    const NATIVE_DENOM: &str = "denom";
    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1),
                    }],
                )
                .unwrap();
        })
    }

    fn timealarms_instantiate(app: &mut App) -> CwTemplateContract {
        let cw_template_id = app.store_code(timealarms_template());
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &timealarms::msg::InstantiateMsg {},
                &[],
                "timealarms_test",
                None,
            )
            .unwrap();
        CwTemplateContract(cw_template_contract_addr)
    }

    fn proper_instantiate(app: &mut App, timealarms_addr: Addr) -> CwTemplateContract {
        let cw_template_id = app.store_code(contract_template());
        let mut msg = dummy_default_instantiate_msg();
        msg.timealarms_addr = timealarms_addr.to_string();
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();
        CwTemplateContract(cw_template_contract_addr)
    }
    mod register_feeder {
        // use super::*;
        // use crate::msg::ExecuteMsg;
        use super::{proper_instantiate, ADMIN, USER};
        use crate::{
            msg::ExecuteMsg,
            tests::integration_tests::tests::{mock_app, timealarms_instantiate},
        };
        use cosmwasm_std::{Addr, Decimal, Timestamp};
        use cw_multi_test::Executor;
        use marketprice::feed::{Price, Prices};
        use std::str::FromStr;
        //TODO: remove after proper implementation of loan SC
        /// The mock for loan SC. It mimics the scheme for time notification.
        /// If GATE, it returns Ok on notifications, returns Err otherwise.
        mod mock_loan {

            use crate::tests::integration_tests::tests::CwTemplateContract;

            use super::ADMIN;
            use cosmwasm_std::{
                Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
                StdResult, Timestamp,
            };
            use cw_multi_test::{App, Contract, ContractWrapper, Executor};
            use cw_storage_plus::Item;
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};
            const GATE: Item<bool> = Item::new("alarm gate");
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
            #[serde(rename_all = "snake_case")]
            pub enum MockExecuteMsg {
                // mimic the scheme
                Alarm(Timestamp),
                // setup GATE
                Gate(bool),
            }
            fn instantiate(deps: DepsMut, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
                GATE.save(deps.storage, &true)?;
                Ok(Response::new().add_attribute("method", "instantiate"))
            }
            fn execute(
                deps: DepsMut,
                _: Env,
                _: MessageInfo,
                msg: MockExecuteMsg,
            ) -> StdResult<Response> {
                match msg {
                    MockExecuteMsg::Alarm(time) => {
                        let gate = GATE.load(deps.storage).expect("storage problem");
                        if gate {
                            Ok(Response::new().add_attribute("loan_reply", time.to_string()))
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
            fn contract_template() -> Box<dyn Contract<Empty>> {
                let contract = ContractWrapper::new(execute, instantiate, query);
                Box::new(contract)
            }
            pub fn proper_instantiate(app: &mut App) -> CwTemplateContract {
                let cw_template_id = app.store_code(contract_template());
                let cw_template_contract_addr = app
                    .instantiate_contract(
                        cw_template_id,
                        Addr::unchecked(ADMIN),
                        &Empty {},
                        &[],
                        "test",
                        None,
                    )
                    .unwrap();
                CwTemplateContract(cw_template_contract_addr)
            }
        }
        #[test]
        fn register_feeder() {
            let mut app = mock_app();
            let cw_template_contract = proper_instantiate(&mut app, Addr::unchecked("timealarms"));
            // only admin can register new feeder, other user should result in error
            let msg = ExecuteMsg::RegisterFeeder {
                feeder_address: USER.to_string(),
            };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(USER), cosmos_msg).unwrap_err();
            // check if admin can register new feeder
            let msg = ExecuteMsg::RegisterFeeder {
                feeder_address: ADMIN.to_string(),
            };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();
        }
        #[test]
        fn test_time_notify() {
            let mut app = mock_app();
            let timealarms = timealarms_instantiate(&mut app);
            app.update_block(cw_multi_test::next_block);

            // instantiate oracle, register feeder
            let oracle = proper_instantiate(&mut app, timealarms.addr());
            let msg = ExecuteMsg::RegisterFeeder {
                feeder_address: ADMIN.to_string(),
            };
            app.execute_contract(Addr::unchecked(ADMIN), oracle.addr(), &msg, &[])
                .unwrap();
            let feed_msg = ExecuteMsg::FeedPrices {
                prices: vec![Prices {
                    base: "A".into(),
                    values: vec![
                        Price::new(Decimal::from_str("100").unwrap(), "B".into()),
                        Price::new(Decimal::from_str("200").unwrap(), "C".into()),
                    ],
                }],
            };
            app.update_block(|bl| bl.time = Timestamp::from_nanos(0));
            // instantiate loan, add alarms
            let loan = mock_loan::proper_instantiate(&mut app);
            let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
                time: Timestamp::from_seconds(1),
            };
            app.execute_contract(loan.addr(), timealarms.addr(), &alarm_msg, &[])
                .unwrap();
            let alarm_msg = timealarms::msg::ExecuteMsg::AddAlarm {
                time: Timestamp::from_seconds(6),
            };
            app.execute_contract(loan.addr(), timealarms.addr(), &alarm_msg, &[])
                .unwrap();
            // advance by 5 seconds
            app.update_block(cw_multi_test::next_block);
            // trigger notification, the GATE is open, events are stacked for the whole chain of contracts calls
            let resp = app
                .execute_contract(Addr::unchecked(ADMIN), oracle.addr(), &feed_msg, &[])
                .unwrap();
            let attr = resp
                .events
                .iter()
                .flat_map(|ev| &ev.attributes)
                .find(|atr| atr.key == "loan_reply")
                .unwrap();
            assert_eq!(attr.value, app.block_info().time.to_string());
            app.update_block(cw_multi_test::next_block);
            // close the GATE, loan return error on notification
            let close_gate = mock_loan::MockExecuteMsg::Gate(false);
            app.execute_contract(Addr::unchecked(ADMIN), loan.addr(), &close_gate, &[])
                .unwrap();
            let resp = app
                .execute_contract(Addr::unchecked(ADMIN), oracle.addr(), &feed_msg, &[])
                .unwrap();
            let attr = resp
                .events
                .iter()
                .flat_map(|ev| &ev.attributes)
                .find(|atr| atr.key == "alarm")
                .unwrap();
            assert_eq!(attr.value, "error");
            // open the GATE, check for remaining alarm
            let open_gate = mock_loan::MockExecuteMsg::Gate(true);
            app.execute_contract(Addr::unchecked(ADMIN), loan.addr(), &open_gate, &[])
                .unwrap();
            let resp = app
                .execute_contract(Addr::unchecked(ADMIN), oracle.addr(), &feed_msg, &[])
                .unwrap();
            let attr = resp
                .events
                .iter()
                .flat_map(|ev| &ev.attributes)
                .find(|atr| atr.key == "loan_reply")
                .unwrap();
            assert_eq!(attr.value, app.block_info().time.to_string());
        }
    }
}
