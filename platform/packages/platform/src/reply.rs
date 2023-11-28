//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use prost::Message;
use serde::de::DeserializeOwned;

use sdk::{
    cosmos_sdk_proto::cosmwasm::wasm::v1::{
        MsgExecuteContractResponse, MsgInstantiateContract2Response, MsgInstantiateContractResponse,
    },
    cosmwasm_std::{from_json, Addr, Api, Binary, Reply},
};

use crate::{error::Error, result::Result};

pub struct InstantiateResponse<T> {
    pub address: Addr,
    pub data: T,
}

impl InstantiateResponse<Vec<u8>> {
    fn into_addr(self) -> Addr {
        self.address
    }
}

pub fn from_instantiate_addr_only(api: &dyn Api, reply: Reply) -> Result<Addr> {
    from_instantiate_inner::<MsgInstantiateContractResponse>(api, reply)
        .map(InstantiateResponse::into_addr)
}

pub fn from_instantiate2_addr_only(api: &dyn Api, reply: Reply) -> Result<Addr> {
    from_instantiate_inner::<MsgInstantiateContract2Response>(api, reply)
        .map(InstantiateResponse::into_addr)
}

pub fn from_execute<T>(reply: Reply) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    decode::<MsgExecuteContractResponse>(reply)
        .map(|data| data.data)
        .and_then(|data| from_json(Binary(data)).map_err(From::from))
}

trait InstantiationResponse
where
    Self: Message + Default + Sized,
{
    fn addr(&self) -> &str;

    fn into_data(self) -> Vec<u8>;
}

impl InstantiationResponse for MsgInstantiateContractResponse {
    fn addr(&self) -> &str {
        &self.address
    }

    fn into_data(self) -> Vec<u8> {
        self.data
    }
}

impl InstantiationResponse for MsgInstantiateContract2Response {
    fn addr(&self) -> &str {
        &self.address
    }

    fn into_data(self) -> Vec<u8> {
        self.data
    }
}

fn from_instantiate_inner<R>(api: &dyn Api, reply: Reply) -> Result<InstantiateResponse<Vec<u8>>>
where
    R: InstantiationResponse,
{
    let response: R = decode(reply)?;

    api.addr_validate(response.addr())
        .map(|address: Addr| InstantiateResponse {
            address,
            data: response.into_data(),
        })
        .map_err(Into::into)
}

fn decode_raw<M>(message: &[u8]) -> Result<M>
where
    M: Message + Default,
{
    M::decode(message).map_err(Into::into)
}

fn decode<M>(reply: Reply) -> Result<M>
where
    M: Message + Default,
{
    reply
        .result
        .into_result()
        .map_err(Error::ReplyResultError)?
        .data
        .ok_or(Error::EmptyReply())
        .map_err(Into::into)
        .and_then(|ref data| decode_raw(data))
}
