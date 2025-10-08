//! For Protobuf layouts refer to [this source](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/tx.proto).
//!
//! Here are defined wrappers for deserializing such structures.

use serde::de::DeserializeOwned;

use sdk::{
    cosmos_sdk_proto::{
        cosmwasm::wasm::v1::{
            MsgExecuteContractResponse, MsgInstantiateContract2Response,
            MsgInstantiateContractResponse,
        },
        prost::Message,
    },
    cosmwasm_std::{Addr, Api, Reply, SubMsgResponse, from_json},
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
    decode_first_response::<MsgExecuteContractResponse>(reply)
        .and_then(|data| from_json(data.data).map_err(Error::serialization))
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
    let response: R = decode_first_response(reply)?;

    api.addr_validate(response.addr())
        .map_err(|err| Error::cosm_wasm_address_invalid(response.addr().to_string(), err))
        .map(|address: Addr| InstantiateResponse {
            address,
            data: response.into_data(),
        })
}

fn decode_raw<M>(message: &[u8]) -> Result<M>
where
    M: Message + Default,
{
    M::decode(message).map_err(Into::into)
}

fn decode_first_response<M>(reply: Reply) -> Result<M>
where
    M: Message + Default,
{
    reply
        .result
        .into_result()
        .map_err(Error::ReplyResultError)
        .and_then(|ref resp| decode_from_responses(resp).or_else(|_| decode_from_data(resp)))
}

// TODO remove once the [PR][https://github.com/CosmWasm/cw-multi-test/issues/206] got released
fn decode_from_data<M>(resp: &SubMsgResponse) -> Result<M>
where
    M: Message + Default,
{
    #[allow(deprecated)]
    resp.data
        .as_ref()
        .ok_or(Error::EmptyReply())
        .and_then(|data| decode_raw(data))
}

fn decode_from_responses<M>(resp: &SubMsgResponse) -> Result<M>
where
    M: Message + Default,
{
    if let [response] = resp.msg_responses.as_slice() {
        decode_raw(&response.value)
    } else {
        Err(Error::EmptyReply())
    }
}
