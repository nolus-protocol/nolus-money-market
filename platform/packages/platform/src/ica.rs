use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::neutron_sdk::bindings::msg::{IbcFee, NeutronMsg};

use crate::{batch::Batch, error::Error, result::Result, trx::Transaction};

#[cfg(not(feature = "testing"))]
use self::impl_::OpenAckVersion;
#[cfg(feature = "testing")]
pub use self::impl_::OpenAckVersion;

/// Identifier of the ICA account opened by a lease
/// It is unique for a lease and allows the support of multiple accounts per lease
const ICA_ACCOUNT_ID: &str = "0";

/// ICA Host Account
///
/// Holds the address on the ICA host network
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct HostAccount(String);

/// Error response to an ICA request
///
/// Contains an unstructured text, that is helpful for manual troubleshooting.
pub struct ErrorResponse {
    details: String,
}

impl TryFrom<String> for HostAccount {
    type Error = Error;
    fn try_from(addr: String) -> Result<Self> {
        if addr.is_empty() {
            Err(Error::InvalidICAHostAccount())
        } else {
            Ok(Self(addr))
        }
    }
}

impl From<HostAccount> for String {
    fn from(account: HostAccount) -> Self {
        account.0
    }
}

impl Display for HostAccount {
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

pub fn register_account<C>(connection: C) -> Batch
where
    C: Into<String>,
{
    let mut batch = Batch::default();
    batch.schedule_execute_no_reply(NeutronMsg::register_interchain_account(
        connection.into(),
        ICA_ACCOUNT_ID.into(),
        None,
    ));
    batch
}

pub fn parse_register_response(response: &str) -> Result<HostAccount> {
    sdk::cosmwasm_std::from_json::<OpenAckVersion>(response)
        .map_err(Error::Deserialization)
        .and_then(|open_ack| open_ack.address.try_into())
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

    batch.schedule_execute_no_reply(NeutronMsg::submit_tx(
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

mod impl_ {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "snake_case")]
    pub struct OpenAckVersion {
        pub version: String,
        pub controller_connection_id: String,
        pub host_connection_id: String,
        pub address: String,
        pub encoding: String,
        pub tx_type: String,
    }
}
