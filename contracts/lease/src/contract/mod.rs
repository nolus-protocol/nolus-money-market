use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use dex::{Account, ConnectionParams, DexConnectable};
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{api::StateResponse, error::ContractResult, lease::LeaseDTO};

pub use self::endpoins::{execute, instantiate, migrate, query, reply, sudo};
use self::state::State;

mod api;
mod cmd;
mod endpoins;
pub mod msg;
mod state;

#[enum_dispatch]
pub(crate) trait Contract {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
}

impl Lease {
    fn new(lease: LeaseDTO, dex: Account) -> Self {
        Self { lease, dex }
    }
}

impl DexConnectable for Lease {
    fn dex(&self) -> &ConnectionParams {
        self.dex.dex()
    }
}
