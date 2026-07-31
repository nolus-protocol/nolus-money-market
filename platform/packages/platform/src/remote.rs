use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use crate::{error::Error, result::Result};

/// Remote Chain Account
///
/// Holds the address on the remote chain
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account(String);

/// Why a remote-chain operation failed, as the remote side described it
///
/// Contains an unstructured text, that is helpful for manual troubleshooting.
/// Deliberately says nothing about which transport carried the failure — an
/// interchain account and the remote-lease controller both report through it.
pub struct ErrorDetails {
    details: String,
}

#[cfg(feature = "testing")]
impl AsRef<str> for Account {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Account {
    type Error = Error;
    fn try_from(addr: String) -> Result<Self> {
        if addr.is_empty() {
            Err(Error::InvalidICAHostAccount())
        } else {
            Ok(Self(addr))
        }
    }
}

impl From<Account> for String {
    fn from(account: Account) -> Self {
        account.0
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(self.0.as_str())
    }
}

impl Display for ErrorDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("remote error with details '{}'", self.details))
    }
}

impl From<String> for ErrorDetails {
    fn from(details: String) -> Self {
        Self { details }
    }
}
