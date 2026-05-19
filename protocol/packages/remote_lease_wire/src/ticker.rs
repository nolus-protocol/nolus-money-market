use std::fmt;

use serde::{Deserialize, Serialize};

/// Bare-string currency ticker as it travels on the wire.
///
/// Serialises transparently as a JSON string. The wire crate does not validate
/// the value against any group; Nolus-side consumers convert into the typed
/// [`currency::CurrencyDTO`] surface, which enforces the compile-time registry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Ticker(String);

impl Ticker {
    pub fn new<S>(value: S) -> Self
    where
        S: Into<String>,
    {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
