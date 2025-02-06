use std::fmt::Debug;

use thiserror::Error;

use currency::{CurrencyDef, SymbolStatic};
use sdk::{
    cosmos_sdk_proto::prost::DecodeError,
    cosmwasm_std::{Addr, Api, StdError},
};

use crate::contract::CodeId;

// TODO replace SymbolStatic and SymbolOwned with CurrencyDTO<G> where approptiate, i.e. the string represent a currency
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Platform] Expecting funds of {0} but found none")]
    NoFunds(SymbolStatic),

    #[error("[Platform] Expecting funds but found none")]
    NoFundsAny(),

    #[error("[Platform] Expecting funds of {0} but found extra ones")]
    UnexpectedFunds(SymbolStatic),

    #[error("[Platform] Expecting funds consisting of a single coin but found more coins")]
    UnexpectedFundsAny(),

    #[error("[Platform] Expecting code id {0} for the contract {1}")]
    UnexpectedCode(String, String),

    #[error("[Platform] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Platform] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Platform] [Std] An error occured while querying code info: {0}")]
    CosmWasmQueryCodeInfo(StdError),

    #[error("[Platform] [Std] An error occured while querying contract info: {0}")]
    CosmWasmQueryContractInfo(StdError),

    #[error("[Platform] [Std] An error occured while querying a currency balance: {0}")]
    CosmWasmAddressInvalid(String, StdError),

    #[error("[Platform] [Std] An error occured while querying a currency balance: {0}")]
    CosmWasmQueryBalance(StdError),

    #[error("[Platform] [Std] An error occured while querying all balances: {0}")]
    CosmWasmQueryAllBalances(StdError),

    #[error("[Platform] [Std] An error occured on data serialization: {0}")]
    Serialization(StdError),

    #[error("[Platform] [Std] An error occured on data deserialization: {0}")]
    Deserialization(StdError),

    #[error("[ICA] Invalid ICA host account")]
    InvalidICAHostAccount(),

    #[error("[Platform] [ProtobufDecode] {0}")]
    ProtobufDecode(#[from] DecodeError),

    #[error("[Platform] Got message type {1} instead of {0}")]
    ProtobufInvalidType(String, String),

    #[error("[Platform] Error returned in reply! Cause: {0}")]
    ReplyResultError(String),

    #[error("[Platform] Reply is empty!")]
    EmptyReply(),
}

impl Error {
    pub fn no_funds<C>() -> Self
    where
        C: CurrencyDef,
    {
        Self::NoFunds(currency::to_string(C::dto()))
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: CurrencyDef,
    {
        Self::UnexpectedFunds(currency::to_string(C::dto()))
    }

    pub fn unexpected_code<A>(exp_code_id: CodeId, instance: A) -> Self
    where
        A: Into<Addr>,
    {
        Self::UnexpectedCode(exp_code_id.to_string(), instance.into().into())
    }
}

pub fn log<Err>(api: &dyn Api) -> impl FnOnce(&Err) + '_
where
    Err: Debug,
{
    |err| api.debug(&format!("{:?}", err))
}
