use std::fmt::Display;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{DepsMut, Env, MessageInfo},
    cw2::set_contract_version,
};

use crate::{api::NewLeaseForm, error::ContractResult};

use super::{Controller, RequestLoan, Response};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
pub struct NoLease {}

impl Controller for NoLease {
    fn instantiate(
        self,
        deps: &mut DepsMut,
        _env: Env,
        info: MessageInfo,
        form: NewLeaseForm,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let (batch, next_state) = RequestLoan::new(deps, info, form)?;

        Ok(Response::from(batch, next_state))
    }
}

impl Display for NoLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease not opened")
    }
}
