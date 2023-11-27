//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use prost::Message;
use serde::de::DeserializeOwned;

use sdk::{
    cosmos_sdk_proto::cosmwasm::wasm::v1::{
        MsgExecuteContractResponse, MsgInstantiateContract2Response, MsgInstantiateContractResponse,
    },
    cosmwasm_std::{from_json, Addr, Api, Binary, Reply, StdError, StdResult},
};

pub struct InstantiateResponse<T> {
    pub address: Addr,
    pub data: T,
}

impl InstantiateResponse<Vec<u8>> {
    fn into_addr(self) -> Addr {
        self.address
    }
}

pub fn from_instantiate_addr_only(api: &dyn Api, reply: Reply) -> StdResult<Addr> {
    from_instantiate_inner::<MsgInstantiateContractResponse>(api, reply)
        .map(InstantiateResponse::into_addr)
}

pub fn from_instantiate2_addr_only(api: &dyn Api, reply: Reply) -> StdResult<Addr> {
    from_instantiate_inner::<MsgInstantiateContract2Response>(api, reply)
        .map(InstantiateResponse::into_addr)
}

pub fn from_execute<T>(reply: Reply) -> StdResult<Option<T>>
where
    T: DeserializeOwned,
{
    decode::<MsgExecuteContractResponse>(reply)
        .map(|data| data.data)
        .map(Binary)
        .and_then(from_json)
}

struct UncheckedInstantiateResponse {
    address: String,
    data: Vec<u8>,
}

impl From<MsgInstantiateContractResponse> for UncheckedInstantiateResponse {
    fn from(
        MsgInstantiateContractResponse { address, data }: MsgInstantiateContractResponse,
    ) -> Self {
        Self { address, data }
    }
}

impl From<MsgInstantiateContract2Response> for UncheckedInstantiateResponse {
    fn from(
        MsgInstantiateContract2Response { address, data }: MsgInstantiateContract2Response,
    ) -> Self {
        Self { address, data }
    }
}

fn from_instantiate_inner<R>(api: &dyn Api, reply: Reply) -> StdResult<InstantiateResponse<Vec<u8>>>
where
    R: Message + Default + Into<UncheckedInstantiateResponse>,
{
    let UncheckedInstantiateResponse { address, data } = decode::<R>(reply)?.into();

    api.addr_validate(&address)
        .map(|address: Addr| InstantiateResponse { address, data })
}

fn decode_raw<M>(message: &[u8]) -> StdResult<M>
where
    M: Message + Default,
{
    M::decode(message).map_err(|error| {
        StdError::generic_err(format!("[Platform] Data is malformed or doesn't comply with used protobuf format! Cause: [Protobuf] {error}"))
    })
}

fn decode<M>(reply: Reply) -> StdResult<M>
where
    M: Message + Default,
{
    decode_raw(
        reply
            .result
            .into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("Reply doesn't contain data!"))?
            .0
            .as_slice(),
    )
}
