use serde::{Deserialize, Serialize};

use dex::{Account, ConnectionParams, DexConnectable};

use crate::lease::LeaseDTO;

pub use self::endpoins::{execute, instantiate, migrate, query, reply, sudo};

mod api;
mod cmd;
mod endpoins;
pub mod msg;
mod state;

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
