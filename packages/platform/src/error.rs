use prost::DecodeError;
use thiserror::Error;

use currency::Currency;
use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Platform] Expecting funds of {0} but found none")]
    NoFunds(String),

    #[error("[Platform] Expecting funds but found none")]
    NoFundsAny(),

    #[error("[Platform] Expecting funds of {0} but found extra ones")]
    UnexpectedFunds(String),

    #[error("[Platform] Expecting funds consisting of a single coin but found more coins")]
    UnexpectedFundsAny(),

    #[error("[Platform] Expecting code id {0} for the contract {1}")]
    UnexpectedCode(String, String),

    #[error("[Platform] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Platform] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Platform] [Std] {0}")]
    CosmWasmError(#[from] StdError),

    #[error("[ICA] Invalid ICA host account")]
    InvalidICAHostAccount(),

    #[error("[ICA] [Deserialization] {0}")]
    Deserialization(#[from] serde_json_wasm::de::Error),

    #[error("[Platform] [ProtobufDecode] {0}")]
    ProtobufDecode(#[from] DecodeError),

    #[error("[Platform] Got message type {1} instead of {0}")]
    ProtobufInvalidType(String, String),
}

impl Error {
    pub fn no_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::NoFunds(C::TICKER.into())
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::UnexpectedFunds(C::TICKER.into())
    }

    pub fn unexpected_code<A>(exp_code_id: u64, instance: A) -> Self
    where
        A: Into<Addr>,
    {
        Self::UnexpectedCode(exp_code_id.to_string(), instance.into().into())
    }
}

pub type Result<T> = core::result::Result<T, Error>;
