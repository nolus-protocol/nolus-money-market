use serde::Deserialize;

use dex::Account;

use crate::{finance::ReserveRef, lease::v8::LeaseDTO};

use super::{finalize::FinalizerRef, Lease as Lease_v9};

#[derive(Deserialize)]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
    finalizer: FinalizerRef,
}

impl Lease {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> Lease_v9 {
        Lease_v9::new(self.lease.migrate(reserve), self.dex, self.finalizer)
    }
}
