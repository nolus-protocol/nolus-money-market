use serde::{Deserialize, Serialize};

use dex::{Account, ConnectionParams, DexConnectable};

use crate::lease::LeaseDTO;

pub use self::endpoins::{execute, instantiate, migrate, query, reply, sudo};
use self::finalize::FinalizerRef;

mod api;
mod cmd;
mod endpoins;
mod finalize;
pub mod msg;
mod state;

#[derive(Serialize, Deserialize)]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
    finalizer: FinalizerRef,
}

impl Lease {
    fn new(lease: LeaseDTO, dex: Account, finalizer: FinalizerRef) -> Self {
        Self {
            lease,
            dex,
            finalizer,
        }
    }
}

impl DexConnectable for Lease {
    fn dex(&self) -> &ConnectionParams {
        self.dex.dex()
    }
}
