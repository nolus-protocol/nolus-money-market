use cosmwasm_std::{Addr, StdError};
use cw_multi_test::{App, Executor};

use rewards_dispatcher::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::tvl_intervals::{Intervals, Stop}
};

use crate::common::ContractWrapper;

use super::ADMIN;

pub struct DispatcherWrapper {
    contract_wrapper: Box<DispatcherContractWrapper>,
}

impl DispatcherWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        lpp: &Addr,
        oracle: &Addr,
        timealarms: &Addr,
        treasury: &Addr,
        _denom: &str,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
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

type DispatcherContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    StdError,
>;
