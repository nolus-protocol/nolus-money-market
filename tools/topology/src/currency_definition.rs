use crate::symbol::Symbol;

#[derive(Debug, PartialEq, Eq)]
pub struct CurrencyDefinition {
    ticker: String,
    host: Symbol,
    dex: Symbol,
    decimal_digits: u8,
}

impl CurrencyDefinition {
    #[inline]
    pub(crate) const fn new(ticker: String, host: Symbol, dex: Symbol, decimal_digits: u8) -> Self {
        Self {
            ticker,
            host,
            dex,
            decimal_digits,
        }
    }

    #[must_use]
    pub fn ticker(&self) -> &str {
        &self.ticker
    }

    #[must_use]
    pub const fn host(&self) -> &Symbol {
        &self.host
    }

    #[must_use]
    pub const fn dex(&self) -> &Symbol {
        &self.dex
    }

    #[must_use]
    pub const fn decimal_digits(&self) -> u8 {
        self.decimal_digits
    }
}
