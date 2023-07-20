use sdk::cosmwasm_std::Timestamp;
use serde::Deserialize;

use crate::{
    contract::state::{
        v4::{Lease, Migrate},
        Response,
    },
    error::ContractResult,
};

use super::active::Active as ActiveV3;

#[derive(Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Migrate for Active {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(ActiveV3::new(self.lease.into())))
    }
}
