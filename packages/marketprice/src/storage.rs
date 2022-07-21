use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::{fraction::Fraction, fractionable::HigherRank, ratio::Rational};

pub type Denom = String;
pub type DenomPair = (Denom, Denom);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct CoinStorage {
    pub amount: u128,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, JsonSchema)]
pub struct PriceStorage {
    base: CoinStorage,
    quote: CoinStorage,
}

impl PriceStorage {
    pub fn new(symbol1: String, base: u128, symbol2: String, quote: u128) -> Self {
        Self::new_from_coins(
            CoinStorage {
                amount: base,
                symbol: symbol1,
            },
            CoinStorage {
                amount: quote,
                symbol: symbol2,
            },
        )
    }

    pub fn new_from_coins(base: CoinStorage, quote: CoinStorage) -> Self {
        PriceStorage { base, quote }
    }

    pub fn base(&self) -> CoinStorage {
        self.base.clone()
    }

    pub fn quote(&self) -> CoinStorage {
        self.quote.clone()
    }

    pub fn one(symbol: &str) -> Self {
        PriceStorage {
            base: CoinStorage {
                amount: 1,
                symbol: symbol.into(),
            },
            quote: CoinStorage {
                amount: 1,
                symbol: symbol.into(),
            },
        }
    }

    pub fn inv(&self) -> Self {
        PriceStorage {
            base: self.quote.clone(),
            quote: self.base.clone(),
        }
    }

    pub fn total(&self, of: &CoinStorage) -> CoinStorage {
        assert_eq!(self.base.symbol, of.symbol);
        let ratio = Rational::new(of.amount, self.base.amount);
        let amount = <Rational<u128> as Fraction<u128>>::of(&ratio, self.quote.amount);
        CoinStorage {
            amount,
            symbol: self.quote.symbol.clone(),
        }
    }

    pub fn denom_pair(&self) -> DenomPair {
        (self.base.symbol.clone(), self.quote.symbol.clone())
    }

    pub fn is_same_type(&self, other: &Self) -> bool {
        self.base.symbol == other.base.symbol && self.quote.symbol == other.quote.symbol
    }
}

impl PartialOrd for PriceStorage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        type DoubleType = <u128 as HigherRank<u128>>::Type;

        let a: DoubleType = self.quote.amount.into();
        let d: DoubleType = other.base.amount.into();

        let b: DoubleType = self.base.amount.into();
        let c: DoubleType = other.quote.amount.into();
        (a * d).partial_cmp(&(b * c))
    }
}
