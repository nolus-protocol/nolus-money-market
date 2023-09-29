use serde::Deserialize;

use crate::contract::{finalize::FinalizerRef, v5::Lease};

use super::active::Active as Active_v6;

#[derive(Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(crate) fn migrate(self, finalizer: FinalizerRef) -> Active_v6 {
        Active_v6::new(self.lease.migrate(finalizer))
    }
}
