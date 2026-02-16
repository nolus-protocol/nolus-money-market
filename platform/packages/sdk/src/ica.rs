use serde::{Deserialize, Serialize};

use cosmwasm_std::{CosmosMsg, CustomMsg};
use schemars::JsonSchema;

#[cfg(feature = "cosmos_proto")]
use super::cosmos_sdk_proto::Any as CosmosAny;
use super::cosmwasm_std::{Binary, Coin as CwCoin};

/// Minimal replacement for `neutron_sdk::bindings::types::ProtobufAny`.
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

// Minimal replacement for `neutron_sdk::sudo::msg::RequestPacketTimeoutHeight`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequestPacketTimeoutHeight {
    pub revision_number: Option<u64>,
    pub revision_height: Option<u64>,
}
