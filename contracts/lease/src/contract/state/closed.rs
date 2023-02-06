use cosmwasm_std::{Deps, Env, QuerierWrapper};
use platform::{
    bank,
    batch::{Emit, Emitter},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{StateQuery, StateResponse},
    contract::cmd::Close,
    error::ContractResult,
    event::Type,
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
    ) -> ContractResult<Emitter> {
        let lease_account = bank::my_account(env, querier);
        let IntoDTOResult { lease: _, batch } =
            with_lease::execute(lease, Close::new(lease_account), querier)?;

        let emitter = batch
            .into_emitter(Type::Close)
            .emit("id", env.contract.address.clone())
            .emit_tx_info(env);

        Ok(emitter)
    }
}

impl Controller for Closed {
    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
