use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use platform::contract::{Code, CodeId, external};
use remote_profit::msg::{CloseProfitParams, OpenProfitParams, SwapParams, TransferOutParams};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Unchecked address of the protocol admin user that can update the profit
    /// code and manage the channel lifecycle.
    pub protocol_admin: String,
    pub connection_id: String,
    pub dex_label: String,
    /// The Solana-side ICS-20 transfer channel paired with this protocol's profit
    /// channel, in canonical `channel-<N>` form. Proposed to the counterparty in
    /// the profit channel's handshake version (ADR-0002 §3.3).
    pub transfer_channel: String,
    pub profit_code: external::Code,
    /// Unchecked address of the single local profit instance this controller
    /// serves. Unlike the multi-instance remote lease — whose addressee rides
    /// each packet envelope — the remote profit is a SINGLETON (ADR-0008): there
    /// is exactly one profit per port/channel, so the callback target is fixed
    /// at instantiation rather than carried on the wire. The controller routes
    /// every `ibc_packet_ack`/`ibc_packet_timeout` callback to this address.
    pub profit_contract: String,
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
    NewProfitCode {
        // This is an internal system API and we use [Code]
        profit_code: Code,
    },
    /// Outbound `OpenProfit` packet. Caller must be an instance of `Config.profit_code`.
    /// `timeout` is the relative duration after which the ICS-04 packet expires;
    /// the controller anchors it to its own block time at send.
    OpenProfit {
        params: OpenProfitParams,
        timeout: Duration,
    },
    /// Outbound `CloseProfit` packet. See [`ExecuteMsg::OpenProfit`] for `timeout` semantics.
    CloseProfit {
        params: CloseProfitParams,
        timeout: Duration,
    },
    /// Outbound `Swap` packet. See [`ExecuteMsg::OpenProfit`] for `timeout` semantics.
    ///
    /// `nonce` is the profit's per-emission correlation identifier; it rides the
    /// packet envelope and is returned in the callback so the profit can match
    /// the acknowledgment to the exact in-flight leg. `#[serde(default)]` keeps
    /// it optional at decode for callers that predate the field.
    Swap {
        params: SwapParams,
        timeout: Duration,
        #[serde(default)]
        nonce: u64,
    },
    /// Outbound `TransferOut` packet. See [`ExecuteMsg::OpenProfit`] for `timeout` semantics.
    ///
    /// `nonce` is the profit's per-emission correlation identifier; it rides the
    /// packet envelope and is returned in the callback so the profit can match
    /// the acknowledgment to the exact in-flight transfer. `#[serde(default)]`
    /// keeps it optional at decode for callers that predate the field.
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
        #[serde(default)]
        nonce: u64,
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
    pub profit_code_id: external::Code,
    pub profit_contract: String,
}

impl ConfigResponse {
    pub fn new(
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        profit_code: Code,
        profit_contract: String,
    ) -> Self {
        Self {
            connection_id,
            dex_label,
            transfer_channel,
            profit_code_id: CodeId::from(profit_code).into(),
            profit_contract,
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
    use platform::{
        contract::{Code, external},
        tests as platform_tests,
    };

    use super::{ConfigResponse, QueryMsg};

    #[test]
    fn release() {
        assert_eq!(
            QueryMsg::ProtocolPackageRelease {},
            platform_tests::ser_de(&versioning::query::ProtocolPackage::Release {}).unwrap(),
        );
    }

    #[test]
    fn config_response_new() {
        let response = ConfigResponse::new(
            "connection-7".into(),
            "osmosis".into(),
            "channel-3".into(),
            Code::unchecked(9),
            "profit".into(),
        );
        assert_eq!("connection-7", response.connection_id);
        assert_eq!("osmosis", response.dex_label);
        assert_eq!("channel-3", response.transfer_channel);
        assert_eq!(external::Code::from(9), response.profit_code_id);
        assert_eq!("profit", response.profit_contract);
    }
}
