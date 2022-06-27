use cosmwasm_std::{Addr, StdError};
use cw_multi_test::ContractWrapper;

use cw_multi_test::{App, Executor};
use profit::ContractError;

use super::ADMIN;

pub struct ProfitWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            profit::msg::ExecuteMsg,
            profit::msg::InstantiateMsg,
            profit::msg::QueryMsg,
            ContractError,
            ContractError,
            StdError,
        >,
    >,
}

impl ProfitWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        cadence_hours: u32,
        treasury: &Addr,
        oracle: &Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = profit::msg::InstantiateMsg {
            cadence_hours,
            treasury: treasury.clone(),
            oracle: oracle.clone(),
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "profit", None)
            .unwrap()
    }
}

impl Default for ProfitWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            profit::contract::execute,
            profit::contract::instantiate,
            profit::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
