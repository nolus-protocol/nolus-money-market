//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use cosmwasm_std::{Addr, Api, Binary, from_binary, Reply, StdError, StdResult};
use serde::de::DeserializeOwned;
use prost::Message;
use serde::Deserialize;

pub struct InstantiateResponse<T> {
    pub address: Addr,
    pub data: Option<T>,
}

pub fn from_instantiate<T>(api: &dyn Api, reply: Reply) -> StdResult<InstantiateResponse<T>>
where
    T: DeserializeOwned,
{
    #[derive(Message)]
    struct ReplyData {
        #[prost(bytes, tag = "1")]
        pub address: Vec<u8>,
        #[prost(bytes, tag = "2")]
        pub data: Vec<u8>,
    }

    let response = decode::<ReplyData>(reply)?;

    Ok(InstantiateResponse {
        address: api.addr_validate(&String::from_utf8(response.address)
            .map_err(|_| StdError::generic_err(
                "Address field contains invalid UTF-8 data!",
            ))?
        )?,
        data: maybe_from_binary(response.data)?,
    })
}

pub fn from_execute<T>(reply: Reply) -> StdResult<Option<T>>
where
    T: DeserializeOwned,
{
    #[derive(prost::Message)]
    struct ReplyData {
        #[prost(bytes, tag = "1")]
        pub data: Vec<u8>,
    }

    decode::<ReplyData>(reply).map(|data| data.data).and_then(maybe_from_binary)
}

fn decode_raw<M>(message: &[u8]) -> StdResult<M>
where
    M: Message + Default,
{
    M::decode(message)
        .map_err(|_| StdError::generic_err(
            "Data is malformed or doesn't comply with used protobuf format!",
        ))
}

fn decode<M>(reply: Reply) -> StdResult<M>
where
    M: Message + Default,
{
    decode_raw(
        reply.result.into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("Reply doesn't contain data!"))?
            .0
            .as_slice(),
    )
}

fn maybe_from_binary<T>(data: Vec<u8>) -> StdResult<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    (!data.is_empty()).then(|| from_binary(&Binary::from(data))).transpose()
}
