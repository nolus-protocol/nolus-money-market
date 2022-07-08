use cosmwasm_std::{Addr, StdError};
use cw_multi_test::ContractWrapper;

use cw_multi_test::{App, Executor};
use timealarms::ContractError;

use super::{mock_query, MockQueryMsg, ADMIN};

pub struct TimeAlarmsWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            timealarms::msg::ExecuteMsg,
            timealarms::msg::InstantiateMsg,
            MockQueryMsg,
            ContractError,
            ContractError,
            StdError,
        >,
    >,
}

impl TimeAlarmsWrapper {
    #[track_caller]
    pub fn instantiate(self, app: &mut App) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = timealarms::msg::InstantiateMsg {};

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
            timealarms::contract::execute,
            timealarms::contract::instantiate,
            mock_query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
