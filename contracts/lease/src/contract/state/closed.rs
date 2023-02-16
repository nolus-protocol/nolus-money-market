use serde::{Deserialize, Serialize};

use platform::{
    bank,
    batch::{Batch, Emit, Emitter},
};
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper};

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
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Batch> {
        let lease_addr = lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        let IntoDTOResult {
            lease: _abandon,
            batch,
        } = with_lease::execute(lease, Close::new(lease_account), querier)?;
        Ok(batch)
    }

    pub(super) fn emit_ok(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl Controller for Closed {
    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
