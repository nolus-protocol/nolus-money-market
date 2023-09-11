use thiserror::Error;

use crate::{
    currency::{Currency, Group, SymbolOwned},
    Symbols,
};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found a symbol '{0}' pretending to be the {1} of the currency with ticker '{2}'")]
    UnexpectedSymbol(String, String, String),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, String, String),
}

impl Error {
    pub fn unexpected_symbol<S, CS, C>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbols + ?Sized,
        C: Currency,
    {
        Self::UnexpectedSymbol(symbol.into(), CS::DESCR.into(), C::TICKER.into())
    }

    pub fn not_in_currency_group<S, CS, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbols + ?Sized,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), CS::DESCR.into(), G::DESCR.into())
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub enum CmdError<CmdErr, ApiErr> {
    Command(CmdErr),
    Api(ApiErr),
}
impl<CmdErr, ApiErr> CmdError<CmdErr, ApiErr> {
    pub fn from_customer_err(err: CmdErr) -> Self {
        Self::Command(err)
    }
    pub fn from_api_err(err: ApiErr) -> Self {
        Self::Api(err)
    }
}
impl<CmdErr, ApiErr> CmdError<CmdErr, ApiErr>
where
    ApiErr: Into<CmdErr>,
{
    pub fn into_customer_err(self) -> CmdErr {
        match self {
            Self::Command(customer_err) => customer_err,
            Self::Api(api_err) => api_err.into(),
        }
    }
}
impl<CmdErr, ApiErr> From<Error> for CmdError<CmdErr, ApiErr>
where
    Error: Into<ApiErr>,
{
    fn from(err: Error) -> Self {
        Self::from_api_err(err.into())
    }
}
