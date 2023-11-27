//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use prost::Message;
use serde::{de::DeserializeOwned, Deserialize};

use sdk::{
    cosmos_sdk_proto::cosmwasm::wasm::v1::{
        MsgExecuteContractResponse, MsgInstantiateContract2Response, MsgInstantiateContractResponse,
    },
    cosmwasm_std::{from_json, Addr, Api, Binary, Reply, StdError, StdResult},
};

pub struct InstantiateResponse<T> {
    pub address: Addr,
    pub data: Option<T>,
}

pub fn from_instantiate<T>(api: &dyn Api, reply: Reply) -> StdResult<InstantiateResponse<T>>
where
    T: DeserializeOwned,
{
    let response: MsgInstantiateContractResponse = decode(reply)?;

    Ok(InstantiateResponse {
        address: api.addr_validate(&response.address)?,
        data: maybe_from_json(response.data)?,
    })
}

pub fn from_instantiate2<T>(api: &dyn Api, reply: Reply) -> StdResult<InstantiateResponse<T>>
where
    T: DeserializeOwned,
{
    let response: MsgInstantiateContract2Response = decode(reply)?;

    Ok(InstantiateResponse {
        address: api.addr_validate(&response.address)?,
        data: maybe_from_json(response.data)?,
    })
}

pub fn from_instantiate2_raw(
    api: &dyn Api,
    reply: Reply,
) -> StdResult<InstantiateResponse<Vec<u8>>> {
    let response: MsgInstantiateContract2Response = decode(reply)?;

    Ok(InstantiateResponse {
        address: api.addr_validate(&response.address)?,
        data: (!response.data.is_empty()).then_some(response.data),
    })
}

pub fn from_execute<T>(reply: Reply) -> StdResult<Option<T>>
where
    T: DeserializeOwned,
{
    decode::<MsgExecuteContractResponse>(reply)
        .map(|data| data.data)
        .and_then(maybe_from_json)
}

fn decode_raw<M>(message: &[u8]) -> StdResult<M>
where
    M: Message + Default,
{
    M::decode(message).map_err(|_| {
        StdError::generic_err("Data is malformed or doesn't comply with used protobuf format!")
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

fn maybe_from_json<T>(data: Vec<u8>) -> StdResult<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    (!data.is_empty())
        .then(|| from_json(Binary::from(data)))
        .transpose()
}
