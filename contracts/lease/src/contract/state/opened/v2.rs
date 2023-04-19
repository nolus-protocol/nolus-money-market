use serde::Deserialize;

use crate::contract::state::{
    v2::{Lease, Migrate},
    State as StateV3,
};

use super::active::Active as ActiveV3;

#[derive(Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Migrate for Active {
    fn into_last_version(self) -> StateV3 {
        ActiveV3::new(self.lease.into()).into()
    }
}
