use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use sdk::schemars::{self, JsonSchema};

pub use protocol_::Protocol;

use crate::{release::Id, Error};

#[cfg(feature = "protocol_contract")]
mod current;
mod protocol_;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct Release {
    id: Id,
    protocol: Protocol,
}

impl Release {
    pub fn check_update_allowed(&self, to: &Self) -> Result<(), Error> {
        if self.protocol == to.protocol {
            Ok(())
        } else {
            Err(Error::protocol_mismatch(&self.protocol, &to.protocol))
        }
    }
}
