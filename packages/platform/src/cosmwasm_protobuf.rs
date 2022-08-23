//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use cosmwasm_std::{from_binary, Reply, StdError, StdResult};
use serde::de::DeserializeOwned;
use prost::Message;

pub struct InstantiateResponse<T> {
    pub address: String,
    pub data: Option<T>,
}

pub fn from_instantiate_reply<T>(reply: Reply) -> StdResult<InstantiateResponse<T>>
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

    let response = <ReplyData as Message>::decode(
        reply.result.into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("Reply doesn't contain data!"))?
            .0
            .as_slice(),
    ).map_err(|_| StdError::generic_err("Data is malformed or doesn't comply with used protobuf format!"))?;

    Ok(InstantiateResponse {
        address: String::from_utf8(response.address)
            .map_err(|_| StdError::generic_err("Address field contains invalid UTF-8 data!"))?,
        data: (!response.data.is_empty()).then(|| from_binary(&response.data.into())).transpose()?,
    })
}

pub fn from_execute_reply<T>(reply: Reply) -> StdResult<T>
where
    T: DeserializeOwned,
{
    #[derive(prost::Message)]
    struct ReplyData {
        #[prost(bytes, tag = "1")]
        pub data: Vec<u8>,
    }

    from_binary(
        &<ReplyData as prost::Message>::decode(
            reply.result.into_result()
                .map_err(StdError::generic_err)?
                .data
                .ok_or_else(|| StdError::generic_err("Reply doesn't contain data!"))?
                .0
                .as_slice(),
        )
            .map_err(|_| StdError::generic_err("Data is malformed or doesn't comply with used protobuf format!"))?
            .data
            .into(),
    )
}
