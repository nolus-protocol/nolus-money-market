use serde::Serialize;

use crate::SymbolStatic;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialOrd, Serialize)]
pub struct Definition {
    /// Identifier of the currency
    pub ticker: SymbolStatic,

    /// Symbol at the Nolus network used by the Cosmos-SDK modules, mainly the Banking one
    pub bank_symbol: SymbolStatic,

    /// Symbol at the Dex network
    pub dex_symbol: SymbolStatic,

    /// Exponent on which the whole unit was raised to get the currency's base
    /// unit represented by the trait.
    ///
    /// Example: `(10 ^ DECIMAL_DIGITS) uUSDC = 1 USDC`
    pub decimal_digits: u8,
}
pub type DefinitionRef = &'static Definition;

impl Definition {
    pub const fn new(
        ticker: SymbolStatic,
        bank: SymbolStatic,
        dex: SymbolStatic,
        decimal_digits: u8,
    ) -> Self {
        Self {
            ticker,
            bank_symbol: bank,
            dex_symbol: dex,
            decimal_digits,
        }
    }
}

impl PartialEq for Definition {
    fn eq(&self, other: &Self) -> bool {
        self.ticker.eq(other.ticker)
    }
}
