use cosmwasm_std::{Addr, StdError};
use cw_multi_test::{App, Executor};

use profit::{
    contract::{execute, instantiate, query},
    ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg}
};

use crate::common::ContractWrapper;

use super::ADMIN;

pub struct ProfitWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            ExecuteMsg,
            ContractError,
            InstantiateMsg,
            ContractError,
            QueryMsg,
            StdError,
        >,
    >,
}

impl ProfitWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        cadence_hours: u16,
        treasury: &Addr,
        timealarms: &Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            cadence_hours,
            treasury: treasury.clone(),
            timealarms: timealarms.clone(),
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "profit", None)
            .unwrap()
    }
}

impl Default for ProfitWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            execute,
            instantiate,
            query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
