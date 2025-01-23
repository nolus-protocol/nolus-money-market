use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

pub use protocol_::Protocol;

use crate::{protocol_name, protocol_network, Error};

mod protocol_;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
// The two usecases, building the current release, and deserializing a release, call for `&'static str` and String, respectively.
// We use Cow since it is an enum of the two. We do not need to mutate it.
pub struct ReleaseId(Cow<'static, str>); //TODO typedef the Cow to `????Str`

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct Release {
    id: ReleaseId,
    protocol: Protocol,
}

impl ReleaseId {
    const ID: &'static str = env!(
        "PROTOCOL_RELEASE_ID",
        "No protocol release identifier provided as an environment variable! Please set \
        \"PROTOCOL_RELEASE_ID\" environment variable!",
    );

    const CURRENT: Self = Self(Cow::Borrowed(Self::ID));
}

impl Release {
    pub const fn current() -> Self {
        Self {
            id: ReleaseId::CURRENT,
            protocol: Protocol::new(protocol_name!(), protocol_network!()),
        }
    }

    pub fn check_update_allowed(&self, to: &Self) -> Result<(), Error> {
        if self.protocol == to.protocol {
            Ok(())
        } else {
            Err(Error::protocol_mismatch(&self.protocol, &to.protocol))
        }
    }
}
