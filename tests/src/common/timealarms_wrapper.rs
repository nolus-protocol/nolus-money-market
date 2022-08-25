use cosmwasm_std::{Addr, StdError};
use cw_multi_test::{App, Executor};

use timealarms::{
    contract::{execute, instantiate, reply},
    ContractError,
    msg::{ExecuteMsg, InstantiateMsg}
};

use crate::common::ContractWrapper;

use super::{ADMIN, mock_query, MockQueryMsg};

pub struct TimeAlarmsWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            ExecuteMsg,
            ContractError,
            InstantiateMsg,
            ContractError,
            MockQueryMsg,
            StdError,
            cosmwasm_std::Empty,
            anyhow::Error,
            ContractError,
        >,
    >,
}

impl TimeAlarmsWrapper {
    #[track_caller]
    pub fn instantiate(self, app: &mut App) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {};

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "timealarms",
            None,
        )
        .unwrap()
    }
}

impl Default for TimeAlarmsWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            execute,
            instantiate,
            mock_query,
        )
            .with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
