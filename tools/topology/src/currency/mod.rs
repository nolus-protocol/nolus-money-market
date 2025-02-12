use std::borrow::Borrow;

use serde::Deserialize;

pub(crate) use self::{ibc::Ibc, native::Native};

mod ibc;
mod native;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct Id(String);

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Id {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(from = "self::RawWithIcon")]
pub(crate) enum Currency {
    Native(Native),
    Ibc(Ibc),
}

impl From<RawWithIcon> for Currency {
    #[inline]
    fn from(RawWithIcon { currency, .. }: RawWithIcon) -> Self {
        match currency {
            Raw::Native(native) => Self::Native(native),
            Raw::Ibc(ibc) => Self::Ibc(ibc),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct RawWithIcon {
    #[serde(flatten)]
    currency: Raw,
    #[serde(rename = "icon")]
    _icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum Raw {
    Native(Native),
    Ibc(Ibc),
}
