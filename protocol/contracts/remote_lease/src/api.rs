use serde::{Deserialize, Serialize};

use platform::contract::{Code, CodeId};
use sdk::cosmwasm_std::Uint64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Unchecked address of the protocol admin user that can update the lease code
    /// and manage the channel lifecycle.
    pub protocol_admin: String,
    pub connection_id: String,
    pub dex_label: String,
    // External system API — accept `Uint64`; the contract wraps it in `Code` after validation.
    pub lease_code: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Internal system API — the validated `Code` wrapper is used.
    NewLeaseCode(Code),
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return a [ConfigResponse]
    Config(),
    /// Return a [ChannelResponse]; `channel` is `None` until the handshake completes.
    Channel(),
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ConfigResponse {
    pub connection_id: String,
    pub dex_label: String,
    pub lease_code_id: Uint64,
}

impl ConfigResponse {
    pub fn new(connection_id: String, dex_label: String, lease_code: Code) -> Self {
        Self {
            connection_id,
            dex_label,
            lease_code_id: CodeId::from(lease_code).into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ChannelStateResponse {
    Open,
    Closing,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ChannelInfo {
    pub local_channel_id: String,
    pub counterparty_channel_id: String,
    pub counterparty_port_id: String,
    pub version: String,
    pub state: ChannelStateResponse,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ChannelResponse {
    pub channel: Option<ChannelInfo>,
}

#[cfg(test)]
mod test {
    use platform::tests as platform_tests;

    use super::QueryMsg;

    #[test]
    fn release() {
        assert_eq!(
            QueryMsg::ProtocolPackageRelease {},
            platform_tests::ser_de(&versioning::query::ProtocolPackage::Release {}).unwrap(),
        );
    }
}
