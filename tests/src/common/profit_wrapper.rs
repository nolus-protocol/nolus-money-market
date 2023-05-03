use profit::{
    contract::{execute, instantiate, query, sudo},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};
use sdk::{
    cosmwasm_std::Addr, cw_multi_test::Executor, neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

pub struct ProfitWrapper {
    contract_wrapper: Box<ProfitContractWrapper>,
}

impl ProfitWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        cadence_hours: u16,
        treasury: Addr,
        oracle: Addr,
        timealarms: Addr,
        connection_id: String,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            cadence_hours,
            treasury,
            oracle,
            timealarms,
            connection_id,
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "profit", None)
            .unwrap()
    }
}

impl Default for ProfitWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query).with_sudo(sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type ProfitContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
    NeutronSudoMsg,
    ContractError,
>;
