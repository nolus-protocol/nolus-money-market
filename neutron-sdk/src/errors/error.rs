use cosmwasm_std::{Decimal256RangeExceeded, DecimalRangeExceeded, OverflowError, StdError};
use serde_json_wasm;
use thiserror::Error;

pub type NeutronResult<T> = Result<T, NeutronError>;

#[derive(Error, Debug, PartialEq)]
pub enum NeutronError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("{0}")]
    FromUTF8Error(#[from] std::string::FromUtf8Error),

    #[error("Bech32 error")]
    Bech32(#[from] bech32::Error),

    #[error("Prost protobuf error")]
    ProstProtobuf(#[from] prost::DecodeError),

    #[error("Serde JSON (Wasm) error")]
    SerdeJSONWasm(String),

    #[error("address length should be max {max:?} bytes, got {actual:?}")]
    MaxAddrLength { max: usize, actual: usize },

    #[error("invalid reply id: {0}")]
    InvalidReplyID(u64),

    #[error("invalid query type: {query_type:?}")]
    InvalidQueryType { query_type: String },

    #[error("Decimal range exceeded")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Decimal256 range exceeded")]
    Decimal256RangeExceeded(#[from] Decimal256RangeExceeded),

    #[error("Overflow error")]
    OverflowError(#[from] OverflowError),

    #[error("Invalid query result format: {0}")]
    InvalidQueryResultFormat(String),

    #[error("Integration tests mock is active")]
    IntegrationTestsMock {},

    #[error("Too many transaction filters, max allowed: {max:?}")]
    TooManyTransactionFilters { max: usize },

    #[error("Can't deconstruct account denom balance key: {0}")]
    AccountDenomBalanceKeyDeconstructionError(String),
}

impl From<serde_json_wasm::de::Error> for NeutronError {
    fn from(e: serde_json_wasm::de::Error) -> Self {
        NeutronError::SerdeJSONWasm(e.to_string())
    }
}
