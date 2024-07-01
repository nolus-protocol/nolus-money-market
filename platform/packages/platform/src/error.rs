use std::fmt::Debug;

use prost::DecodeError;
use thiserror::Error;

use currency::{group::MemberOf, Currency, CurrencyDTO, Group};
use sdk::cosmwasm_std::{Addr, Api, StdError};

use crate::contract::CodeId;

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
    pub fn no_funds<C, G>() -> Self
    where
        C: Currency + MemberOf<G>,
        G: Group,
    {
        Self::NoFunds(CurrencyDTO::display::<C>())
    }

    pub fn unexpected_funds<C, G>() -> Self
    where
        C: Currency + MemberOf<G>,
    {
        Self::UnexpectedFunds(C::TICKER.into())
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
