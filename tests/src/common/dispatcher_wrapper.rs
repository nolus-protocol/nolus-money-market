use cosmwasm_std::{Addr, StdError};
use cw_multi_test::ContractWrapper;

use rewards_dispatcher::state::tvl_intervals::{Intervals, Stop};

use cw_multi_test::{App, Executor};

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
        app: &mut App,
        lpp: &Addr,
        time_oracle: &Addr,
        treasury: &Addr,
        market_oracle: &Addr,
        _denom: &str,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = rewards_dispatcher::msg::InstantiateMsg {
            cadence_hours: 10,
            lpp: lpp.clone(),
            time_oracle: time_oracle.clone(),
            treasury: treasury.clone(),
            market_oracle: market_oracle.clone(),
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
