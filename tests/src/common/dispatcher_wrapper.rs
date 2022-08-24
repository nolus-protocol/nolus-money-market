use cosmwasm_std::{Addr, StdError};
use cw_multi_test::{
    ContractWrapper,
    Executor
};

use rewards_dispatcher::state::tvl_intervals::{Intervals, Stop};

use crate::common::MockApp;

use super::ADMIN;

pub struct DispatcherWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            rewards_dispatcher::msg::ExecuteMsg,
            rewards_dispatcher::msg::InstantiateMsg,
            rewards_dispatcher::msg::QueryMsg,
            rewards_dispatcher::error::ContractError,
            rewards_dispatcher::error::ContractError,
            StdError,
        >,
    >,
}

impl DispatcherWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        lpp: &Addr,
        oracle: &Addr,
        timealarms: &Addr,
        treasury: &Addr,
        _denom: &str,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = rewards_dispatcher::msg::InstantiateMsg {
            cadence_hours: 10,
            lpp: lpp.clone(),
            oracle: oracle.clone(),
            timealarms: timealarms.clone(),
            treasury: treasury.clone(),
            tvl_to_apr: Intervals::from(vec![Stop::new(0, 10), Stop::new(1000000, 10)]).unwrap(),
        };

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "dispatcher",
            None,
        )
        .unwrap()
    }
}

impl Default for DispatcherWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            rewards_dispatcher::contract::execute,
            rewards_dispatcher::contract::instantiate,
            rewards_dispatcher::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
