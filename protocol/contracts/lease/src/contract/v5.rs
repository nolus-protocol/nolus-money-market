use serde::Deserialize;

use dex::Account;

use crate::lease::v5::LeaseDTO;

use super::{finalize::FinalizerRef, Lease as Lease_v6};

#[derive(Deserialize)]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
}

impl Lease {
    pub(crate) fn migrate(self, finalizer: FinalizerRef) -> Lease_v6 {
        Lease_v6::new(self.lease.migrate(), self.dex, finalizer)
    }
}
