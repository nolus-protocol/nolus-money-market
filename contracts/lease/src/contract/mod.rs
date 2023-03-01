use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{dex::ConnectionParams, StateResponse},
    error::ContractResult,
    lease::LeaseDTO,
};

pub use self::state::{execute, instantiate, migrate, query, reply, sudo};
use self::{
    dex::{Account, DexConnectable},
    state::State,
};

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
