use thiserror::Error;

use crate::{
    currency::{Currency, Group, SymbolOwned},
    CurrencySymbol,
};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found bank symbol '{0}' expecting '{1}'")]
    UnexpectedBankSymbol(String, String),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, String, String),
}

impl Error {
    pub fn unexpected_bank_symbol<S, C>(bank_symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        C: Currency,
    {
        Self::UnexpectedBankSymbol(bank_symbol.into(), C::BANK_SYMBOL.into())
    }

    pub fn not_in_currency_group<S, CS, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: CurrencySymbol + ?Sized,
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
