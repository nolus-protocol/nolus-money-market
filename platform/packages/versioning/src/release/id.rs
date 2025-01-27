use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
// The two usecases, building the current release, and deserializing a release, call for `&'static str` and String, respectively.
// We use Cow since it is an enum of the two. We do not need to mutate it.
pub struct Id(Cow<'static, str>); //TODO typedef the Cow to `????Str`

impl Id {
    pub(crate) const fn new_static(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }
}

impl Id {
    pub const VOID: Self = Self(Cow::Borrowed("void-release"));
}

// TODO get rid of when the check in the admin gets removed
impl From<Id> for String {
    fn from(value: Id) -> Self {
        value.0.to_string()
    }
}
