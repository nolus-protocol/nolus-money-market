use serde::Deserialize;

use crate::newtype;

pub(crate) use self::endpoint::Endpoint;

mod endpoint;

newtype::define!(
    #[derive(Debug, Clone, Deserialize)]
    #[serde(transparent)]
    pub(crate) Id(String)
    as [String, str]
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Channel {
    a: Endpoint,
    b: Endpoint,
}

impl Channel {
    #[inline]
    pub const fn a(&self) -> &Endpoint {
        &self.a
    }

    #[inline]
    pub const fn b(&self) -> &Endpoint {
        &self.b
    }
}
