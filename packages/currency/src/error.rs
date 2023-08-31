use thiserror::Error;

use crate::currency::{Currency, Group, SymbolOwned};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Finance] Found bank symbol '{0}' expecting '{1}'")]
    UnexpectedBankSymbol(String, String),

    #[error("[Finance] Found dex symbol '{0}' expecting '{1}'")]
    UnexpectedDexSymbol(String, String),

    #[error("[Finance] Found currency '{0}' which is not defined in the {1} currency group")]
    NotInCurrencyGroup(String, String),
}

impl Error {
    pub fn unexpected_bank_symbol<S, C>(bank_symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        C: Currency,
    {
        Self::UnexpectedBankSymbol(bank_symbol.into(), C::BANK_SYMBOL.into())
    }

    pub fn unexpected_dex_symbol<S, C>(dex_symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        C: Currency,
    {
        Self::UnexpectedDexSymbol(dex_symbol.into(), C::DEX_SYMBOL.into())
    }

    pub fn not_in_currency_group<S, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), G::DESCR.into())
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
