use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper};
use platform::{
    bank,
    batch::{Emit, Emitter},
};

use crate::{
    api::{StateQuery, StateResponse},
    contract::cmd::Close,
    error::ContractResult,
    event::Type,
    lease::{with_lease, IntoDTOResult, LeaseDTO},
};

use super::{Controller, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Closed {
    pub(super) fn enter_state(
        self,
        lease: LeaseDTO,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        let lease_account = bank::account(&env.contract.address, querier);
        let IntoDTOResult { lease: _, batch } =
            with_lease::execute(lease, Close::new(lease_account), querier)?;

        let emitter = Emitter::of_type(Type::Closed)
            .emit("id", env.contract.address.clone())
            .emit_tx_info(env);

        Ok(Response::from(
            batch.into_response(emitter),
            Closed::default(),
        ))
    }
}

impl Controller for Closed {
    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
