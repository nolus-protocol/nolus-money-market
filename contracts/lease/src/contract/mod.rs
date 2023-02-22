use cosmwasm_std::{QuerierWrapper, Timestamp};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{
    api::{dex::ConnectionParams, StateResponse},
    error::ContractResult,
    lease::LeaseDTO,
};

use self::{
    dex::{Account, DexConnectable},
    state::State,
};
pub use state::{execute, instantiate, migrate, query, reply, sudo};

mod cmd;
mod dex;
pub mod msg;
mod state;

#[enum_dispatch]
trait Contract {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
}

impl DexConnectable for Lease {
    fn dex(&self) -> &ConnectionParams {
        self.dex.dex()
    }
}
