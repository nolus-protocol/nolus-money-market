use serde::Deserialize;

pub(crate) use self::endpoint::Endpoint;

mod endpoint;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct Id(String);

impl AsRef<str> for Id {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

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
