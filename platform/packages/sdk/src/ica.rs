use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin as CwCoin, CosmosMsg, CustomMsg};

/// IbcFee defines struct for fees that refund the relayer for `SudoMsg` messages submission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IbcFee {
    pub recv_fee: Vec<CwCoin>,
    pub ack_fee: Vec<CwCoin>,
    pub timeout_fee: Vec<CwCoin>,
}

/// Minimal replacement for `neutron_sdk::sudo::msg::RequestPacketTimeoutHeight`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestPacketTimeoutHeight {
    pub revision_number: Option<u64>,
    pub revision_height: Option<u64>,
}

/// Minimal replacement for `neutron_sdk::bindings::msg::NeutronMsg`.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterChainMsg {
    IbcTransfer {
        source_port: String,
        source_channel: String,
        token: CwCoin,
        sender: String,
        receiver: String,
        timeout_height: RequestPacketTimeoutHeight,
        timeout_timestamp: u64,
        memo: String,
        fee: IbcFee,
    },
}

impl From<InterChainMsg> for CosmosMsg<InterChainMsg> {
    fn from(msg: InterChainMsg) -> Self {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for InterChainMsg {}

// Minimal replacement for `neutron_sdk::sudo::msg::RequestPacket`.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestPacket {
    pub sequence: Option<u64>,
    pub source_port: Option<String>,
    pub source_channel: Option<String>,
    pub destination_port: Option<String>,
    pub destination_channel: Option<String>,
    pub data: Option<Binary>,
    pub timeout_height: Option<RequestPacketTimeoutHeight>,
    pub timeout_timestamp: Option<u64>,
}

// Minimal replacement for `neutron_sdk::sudo::msg::SudoMsg`.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    Response {
        request: RequestPacket,
        data: Binary,
    },
    Error {
        request: RequestPacket,
        details: String,
    },
    Timeout {
        request: RequestPacket,
    },
    OpenAck {
        port_id: String,
        channel_id: String,
        counterparty_channel_id: String,
        counterparty_version: String,
    },
}
