use crate::{msg::InstantiateMsg, tests::helpers::CwTemplateContract};
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

pub fn contract_template() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

const USER: &str = "USER";
const ADMIN: &str = "ADMIN";
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

fn proper_instantiate() -> (App, CwTemplateContract) {
    let mut app = mock_app();
    let cw_template_id = app.store_code(contract_template());

    let msg = InstantiateMsg {
        cadence_hours: 3u16,
        treasury: Addr::unchecked("treasury"),
        timealarms: Addr::unchecked("timealarms"),
    };
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

    let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

    (app, cw_template_contract)
}

mod config {
    use super::*;
    use crate::msg::ExecuteMsg;

    #[test]
    #[should_panic(expected = "ContractData not found")]
    fn config() {
        let (mut app, cw_template_contract) = proper_instantiate();

        let msg = ExecuteMsg::Config {
            cadence_hours: 12u16,
        };
        let cosmos_msg = cw_template_contract.call(msg).unwrap();
        app.execute(Addr::unchecked(USER), cosmos_msg).unwrap_err();
    }
}
