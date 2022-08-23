use cosmwasm_std::{StdResult, from_binary, Reply, StdError, Addr};
use serde::de::DeserializeOwned;

//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

pub fn from_instantiate_reply<T>(reply: Reply) -> StdResult<(Addr, T)>
where
    T: DeserializeOwned,
{
    #[derive(prost::Message)]
    struct ReplyData {
        #[prost(bytes, tag = "1")]
        pub address: Addr,
        #[prost(bytes, tag = "2")]
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
