use serde::{Deserialize, Serialize};

use platform::contract::{Code, CodeId};
use remote_lease::Ics20ChannelId;
use sdk::cosmwasm_std::Uint64;

pub use remote_lease::msg::ExecuteMsg;

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

/// The channel record's phase, as observed from outside.
///
/// The handshake phases are visible so an operator can tell a proposal awaiting
/// its `OpenInit` from one awaiting the counterparty's ack — the two need
/// different interventions, and only the latter is waiting on the counterparty.
///
/// Every phase reports the proposed pairing twice over: `ics20_channel_remote`
/// as the first-class value, and `version` as the exact bytes on the wire. The
/// first is what an operator checks the deployment against; the second is what
/// they diff against the counterparty's own logs.
// Why this mirrors `state::Channel` rather than serialising it directly:
//
// - This module compiles in the API-only build, so a client can deserialise a
//   query response without pulling in the contract stack. `state::Channel` is
//   storage code behind the `contract` feature and would drag it along.
// - Storage layout and query wire format answer to different disciplines — the
//   first breaks on migration, the second breaks clients — so the mirror lets
//   either move without forcing the other. The exhaustive mapping match in
//   `state` is what keeps the two honest: adding a phase there fails to
//   compile until it is reflected here.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ChannelInfo {
    Proposed {
        ics20_channel_remote: Ics20ChannelId,
        version: String,
    },
    InitAccepted {
        ics20_channel_remote: Ics20ChannelId,
        version: String,
        local_channel_id: String,
    },
    Established {
        local_channel_id: String,
        counterparty_channel_id: String,
        counterparty_port_id: String,
        ics20_channel_remote: Ics20ChannelId,
        version: String,
        state: ChannelStateResponse,
    },
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
