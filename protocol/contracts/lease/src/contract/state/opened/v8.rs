use serde::Deserialize;

use crate::{contract::v8::Lease, finance::ReserveRef};

use super::active::Active as Active_v9;

#[derive(Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> Active_v9 {
        Active_v9::new(self.lease.migrate(reserve))
    }
}
