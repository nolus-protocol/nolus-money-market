use sdk::{
    cosmwasm_std::{Addr, Empty, StdError},
    cw_multi_test::Executor,
};
use timealarms::{
    contract::{execute, instantiate, reply},
    msg::{ExecuteMsg, InstantiateMsg},
    ContractError,
};

use crate::common::{ContractWrapper, MockApp};

use super::{mock_query, MockQueryMsg, ADMIN};

pub struct TimeAlarmsWrapper {
    contract_wrapper: Box<TimeAlarmsContractWrapper>,
}

impl TimeAlarmsWrapper {
    #[track_caller]
    pub fn instantiate(self, app: &mut MockApp) -> Addr {
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
        let contract = ContractWrapper::new(execute, instantiate, mock_query).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type TimeAlarmsContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    MockQueryMsg,
    StdError,
    Empty,
    anyhow::Error,
    ContractError,
>;
