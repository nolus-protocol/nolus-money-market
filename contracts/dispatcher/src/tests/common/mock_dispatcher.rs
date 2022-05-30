use cosmwasm_std::{Addr, StdError};
use cw_multi_test::ContractWrapper;

use crate::state::tvl_intervals::{Intervals, Stop};

use cw_multi_test::{App, Executor};

use super::ADMIN;

pub struct MockDispatcher {
    contract_wrapper: Box<
        ContractWrapper<
            crate::msg::ExecuteMsg,
            crate::msg::InstantiateMsg,
            crate::msg::QueryMsg,
            crate::error::ContractError,
            crate::error::ContractError,
            StdError,
        >,
    >,
}

impl MockDispatcher {
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
        let msg = crate::msg::InstantiateMsg {
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

impl Default for MockDispatcher {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
