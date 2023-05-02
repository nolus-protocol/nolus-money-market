use sdk::{
    cosmwasm_std::{Addr, Coin, Uint128},
    testing::{
        new_app, new_custom_msg_queue, App, Contract, ContractWrapper, CustomMessageSender,
        Executor, WrappedCustomMessageReceiver,
    },
};

use crate::{
    msg::{InstantiateMsg, SudoMsg},
    tests::helpers::CwTemplateContract,
};

pub fn contract_template() -> Box<Contract> {
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

fn mock_app(custom_message_sender: CustomMessageSender) -> App {
    new_app(custom_message_sender).build(|router, _, storage| {
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

fn proper_instantiate() -> (App, CwTemplateContract, WrappedCustomMessageReceiver) {
    let (custom_message_sender, custom_message_receiver): (
        CustomMessageSender,
        WrappedCustomMessageReceiver,
    ) = new_custom_msg_queue();
    let mut app = mock_app(custom_message_sender);
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

    (app, cw_template_contract, custom_message_receiver)
}

mod config {
    use super::*;

    #[test]
    #[should_panic(expected = "ContractData not found")]
    fn config() {
        let (mut app, cw_template_contract, _custom_message_receiver): (
            App,
            CwTemplateContract,
            WrappedCustomMessageReceiver,
        ) = proper_instantiate();

        app.wasm_sudo(
            cw_template_contract.addr(),
            &SudoMsg::Config {
                cadence_hours: 12u16,
            },
        )
        .unwrap_err();
    }
}
