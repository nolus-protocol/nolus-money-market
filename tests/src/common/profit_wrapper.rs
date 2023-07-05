use profit::{
    contract::{execute, instantiate, query, sudo},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    typedefs::CadenceHours,
    ContractError,
};
use sdk::{cosmwasm_std::Addr, neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg};

use super::{test_case::App, CwContractWrapper, ADMIN};

pub(crate) struct ProfitWrapper {
    contract_wrapper: Box<ProfitContractWrapper>,
}

impl ProfitWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: Addr,
        timealarms: Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            cadence_hours,
            treasury,
            oracle,
            timealarms,
        };

        app.instantiate(code_id, Addr::unchecked(ADMIN), &msg, &[], "profit", None)
            .unwrap()
            .unwrap_response()
    }
}

impl Default for ProfitWrapper {
    fn default() -> Self {
        let contract = CwContractWrapper::new(execute, instantiate, query).with_sudo(sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type ProfitContractWrapper = CwContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
    NeutronSudoMsg,
    ContractError,
>;
