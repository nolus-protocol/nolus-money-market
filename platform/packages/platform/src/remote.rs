use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::ica::{IbcFee, InterChainMsg};

use crate::{batch::Batch, error::Error, result::Result, trx::Transaction};

/// Identifier of the ICA account opened by a lease
/// It is unique for a lease and allows the support of multiple accounts per lease
const ICA_ACCOUNT_ID: &str = "0";

/// Remote Chain Account
///
/// Holds the address on the remote chain
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account(String);

/// Error response to a remote account request
///
/// Contains an unstructured text, that is helpful for manual troubleshooting.
pub struct ErrorResponse {
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

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("ICA error with details '{}'", self.details))
    }
}

impl From<String> for ErrorResponse {
    fn from(details: String) -> Self {
        Self { details }
    }
}

pub fn submit_transaction<Conn, M>(
    connection: Conn,
    trx: Transaction,
    memo: M,
    timeout: Duration,
) -> Batch
where
    Conn: Into<String>,
    M: Into<String>,
{
    let mut batch = Batch::default();

    batch.schedule_execute_no_reply(InterChainMsg::submit_tx(
        connection.into(),
        ICA_ACCOUNT_ID.into(),
        trx.into_msgs(),
        memo.into(),
        timeout.secs(),
        IbcFee {
            recv_fee: vec![],
            ack_fee: vec![],
            timeout_fee: vec![],
        },
    ));
    batch
}
