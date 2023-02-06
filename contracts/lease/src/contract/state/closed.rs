use cosmwasm_std::{Deps, Env, QuerierWrapper};
use platform::{bank, batch::Batch};
use serde::{Deserialize, Serialize};

use crate::{
    api::{StateQuery, StateResponse},
    contract::cmd::Close,
    error::ContractResult,
    lease::{with_lease, IntoDTOResult, LeaseDTO},
};

use super::Controller;

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Closed {
    pub(super) fn enter_state(
        &self,
        lease: LeaseDTO,
        env: &Env,
        querier: &QuerierWrapper,
    ) -> ContractResult<Batch> {
        let lease_account = bank::my_account(env, querier);
        let IntoDTOResult { lease: _, batch } =
            with_lease::execute(lease, Close::new(lease_account), querier)?;

        Ok(batch)
    }
}

impl Controller for Closed {
    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
