use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin as CwCoin, CosmosMsg, CustomMsg, StdResult};
use schemars::JsonSchema;

#[cfg(feature = "cosmos_proto")]
use super::cosmos_sdk_proto::Any as CosmosAny;

/// Minimal replacement for `neutron_sdk::bindings::types::ProtobufAny`.
///
// TODO: Replace with `ibc_proto::Any` once cosmwasm-std v3 drops the `JsonSchema` requirement.
// Exists only because `ibc_proto::Any` cannot derive `JsonSchema`, which is
// currently forced on `InterChainMsg` by `cw-multi-test`'s `WasmKeeper` trait bounds.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProtobufAny {
    pub type_url: String,
    pub value: Binary,
}

impl ProtobufAny {
    pub fn new(type_url: String, value: Vec<u8>) -> Self {
        Self {
            type_url,
            value: Binary::new(value),
        }
    }
}

#[cfg(feature = "cosmos_proto")]
impl From<ProtobufAny> for CosmosAny {
    fn from(p: ProtobufAny) -> Self {
        Self {
            type_url: p.type_url,
            value: p.value.to_vec(),
        }
    }
}

#[cfg(feature = "cosmos_proto")]
impl From<CosmosAny> for ProtobufAny {
    fn from(a: CosmosAny) -> Self {
        Self {
            type_url: a.type_url,
            value: Binary::new(a.value.to_vec()),
        }
    }
}

/// IbcFee defines struct for fees that refund the relayer for `SudoMsg` messages submission.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IbcFee {
    pub recv_fee: Vec<CwCoin>,
    pub ack_fee: Vec<CwCoin>,
    pub timeout_fee: Vec<CwCoin>,
}

/// Minimal replacement for `neutron_sdk::sudo::msg::RequestPacketTimeoutHeight`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestPacketTimeoutHeight {
    pub revision_number: Option<u64>,
    pub revision_height: Option<u64>,
}

/// Minimal replacement for `neutron_sdk::bindings::msg::NeutronMsg`.
///
// TODO: Remove `JsonSchema` derives and the `schemars` dependency once we upgrade to cosmwasm-std v3.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterChainMsg {
    RegisterAccount {
        connection_id: String,
        ica_id: String,
        register_fee: Option<Vec<CwCoin>>,
    },

    SubmitTx {
        connection_id: String,
        ica_id: String,
        msgs: Vec<ProtobufAny>,
        memo: String,
        timeout: u64,
        fee: IbcFee,
    },
    IbcTransfer {
        source_port: String,
        source_channel: String,
        token: CwCoin,
        sender: String,
        receiver: String,
        timeout_height: RequestPacketTimeoutHeight,
        timeout_timestamp: u64,
        fee: IbcFee,
        memo: String,
    },
}

impl InterChainMsg {
    pub fn register_interchain_account(
        connection_id: String,
        ica_id: String,
        register_fee: Option<Vec<CwCoin>>,
    ) -> Self {
        InterChainMsg::RegisterAccount {
            connection_id,
            ica_id,
            register_fee,
        }
    }

    pub fn submit_tx(
        connection_id: String,
        ica_id: String,
        msgs: Vec<ProtobufAny>,
        memo: String,
        timeout: u64,
        fee: IbcFee,
    ) -> Self {
        InterChainMsg::SubmitTx {
            connection_id,
            ica_id,
            msgs,
            memo,
            timeout,
            fee,
        }
    }
}

impl From<InterChainMsg> for CosmosMsg<InterChainMsg> {
    fn from(msg: InterChainMsg) -> Self {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for InterChainMsg {}

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

impl OpenAckVersion {
    pub fn parse(response: &str) -> StdResult<Self> {
        cosmwasm_std::from_json(response)
    }
}

// Minimal replacement for `neutron_sdk::sudo::msg::RequestPacket`.
#[derive(Clone, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestPacket {
    pub sequence: Option<u64>,
    pub source_port: Option<String>,
    pub source_channel: Option<String>,
    pub destination_port: Option<String>,
    pub destination_channel: Option<String>,
    pub data: Option<Vec<u8>>,
    pub timeout_height: Option<RequestPacketTimeoutHeight>,
    pub timeout_timestamp: Option<u64>,
}

// Minimal replacement for `neutron_sdk::sudo::msg::SudoMsg`.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    OpenAck {
        port_id: String,
        channel_id: String,
        counterparty_channel_id: String,
        counterparty_version: String,
    },
    Response {
        request: RequestPacket,
        data: Binary,
    },
    Timeout {
        request: RequestPacket,
    },
    Error {
        request: RequestPacket,
        details: String,
    },
    TxQueryResult {
        query_id: u64,
    },
    KVQueryResult {
        query_id: u64,
    },
}
