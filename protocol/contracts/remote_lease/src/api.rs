use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use platform::contract::{Code, CodeId};
use remote_lease::msg::{CloseLeaseParams, OpenLeaseParams, SwapParams, TransferOutParams};
use sdk::cosmwasm_std::Uint64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Unchecked address of the protocol admin user that can update the lease code
    /// and manage the channel lifecycle.
    pub protocol_admin: String,
    pub connection_id: String,
    pub dex_label: String,
    /// The Solana-side ICS-20 transfer channel paired with this protocol's lease
    /// channel, in canonical `channel-<N>` form. Proposed to the counterparty in
    /// the lease channel's handshake version (ADR-0002 §3.3).
    pub transfer_channel: String,
    // External system API — accept `Uint64`; the contract wraps it in `Code` after validation.
    pub lease_code: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Initiate the channel handshake. Allowed only when no channel is recorded.
    OpenChannel(),
    /// Begin closing the recorded channel. Allowed only when it is currently `Open`.
    CloseChannel(),
    NewLeaseCode {
        // This is an internal system API and we use [Code]
        lease_code: Code,
    },
    /// Outbound `OpenLease` packet. Caller must be an instance of `Config.lease_code`.
    /// `timeout` is the relative duration after which the ICS-04 packet expires;
    /// the controller anchors it to its own block time at send.
    OpenLease {
        params: OpenLeaseParams,
        timeout: Duration,
    },
    /// Outbound `CloseLease` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    CloseLease {
        params: CloseLeaseParams,
        timeout: Duration,
    },
    /// Outbound `Swap` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    Swap {
        params: SwapParams,
        timeout: Duration,
    },
    /// Outbound `TransferOut` packet. See [`ExecuteMsg::OpenLease`] for `timeout` semantics.
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
    },
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
    pub transfer_channel: String,
    pub lease_code_id: Uint64,
}

impl ConfigResponse {
    pub fn new(
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        lease_code: Code,
    ) -> Self {
        Self {
            connection_id,
            dex_label,
            transfer_channel,
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
