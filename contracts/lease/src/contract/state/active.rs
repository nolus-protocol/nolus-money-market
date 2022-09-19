use std::fmt::Display;

use platform::bank::BankStub;

use crate::{lease::{LeaseDTO, self}, contract::cmd::LeaseState, error::ContractResult};

use super::{Controller, QueryResponse};

pub struct Active {}

impl Controller for Active {
    // add repay, close, and the other execute messages
    // fn execute(
    //     self,
    //     _deps: cosmwasm_std::DepsMut,
    //     _env: cosmwasm_std::Env,
    //     _info: cosmwasm_std::MessageInfo,
    //     _msg: crate::msg::ExecuteMsg,
    // ) -> crate::error::ContractResult<super::Response> {
    //     super::err("execute", &self)
    // }

    fn query(
        self,
        deps: cosmwasm_std::Deps,
        env: cosmwasm_std::Env,
        _msg: crate::msg::StateQuery,
    ) -> ContractResult<QueryResponse> {
        let lease = LeaseDTO::load(deps.storage)?;

        let bank = BankStub::my_account(&env, &deps.querier);

        // TODO think on taking benefit from having a LppView trait
        let resp = lease::execute(
            lease,
            LeaseState::new(env.block.time, bank),
            &env.contract.address,
            &deps.querier,
        )?;
        Ok(QueryResponse::from(resp, self))
    }
}

impl Display for Active {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("active lease")
    }
}
