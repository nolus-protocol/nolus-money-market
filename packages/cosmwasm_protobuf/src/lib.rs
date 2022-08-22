use cosmwasm_std::{StdResult, from_binary, Reply, StdError};
use serde::de::DeserializeOwned;

pub fn from_reply<T>(reply: Reply) -> StdResult<T>
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
