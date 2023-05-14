use serde::Deserialize;

use crate::contract::state::{
    v2::{Lease, Migrate},
    Response,
};

use super::active::Active as ActiveV3;

#[derive(Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Migrate for Active {
    fn into_last_version(self) -> Response {
        Response::no_msgs(ActiveV3::new(self.lease.into()))
    }
}
