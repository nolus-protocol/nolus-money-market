use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::{fraction::Fraction, fractionable::HigherRank, ratio::Rational};

pub type Denom = String;
pub type DenomPair = (Denom, Denom);

#[deprecated = "Migrate to using finance::coin::Coin"]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct Coin {
    pub amount: u128,
    pub symbol: String,
}

#[deprecated = "Migrate to using finance::price::Price"]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, JsonSchema)]
pub struct Price {
    base: Coin,
    quote: Coin,
}

impl Price {
    pub fn new<S1, S2>(symbol1: S1, base: u128, symbol2: S2, quote: u128) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self::new_from_coins(
            Coin {
                amount: base,
                symbol: symbol1.into(),
            },
            Coin {
                amount: quote,
                symbol: symbol2.into(),
            },
        )
    }

    pub fn new_from_coins(base: Coin, quote: Coin) -> Self {
        Price { base, quote }
    }

    pub fn base(&self) -> Coin {
        self.base.clone()
    }

    pub fn quote(&self) -> Coin {
        self.quote.clone()
    }

    pub fn one(symbol: &str) -> Self {
        Price {
            base: Coin {
                amount: 1,
                symbol: symbol.into(),
            },
            quote: Coin {
                amount: 1,
                symbol: symbol.into(),
            },
        }
    }

    pub fn inv(&self) -> Self {
        Price {
            base: self.quote.clone(),
            quote: self.base.clone(),
        }
    }
    pub fn total(&self, of: &Coin) -> Coin {
        assert_eq!(self.base.symbol, of.symbol);
        let ratio = Rational::new(of.amount, self.base.amount);
        let amount = <Rational<u128> as Fraction<u128>>::of(&ratio, self.quote.amount);
        Coin {
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

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        type DoubleType = <u128 as HigherRank<u128>>::Type;

        let a: DoubleType = self.quote.amount.into();
        let d: DoubleType = other.base.amount.into();

        let b: DoubleType = self.base.amount.into();
        let c: DoubleType = other.quote.amount.into();
        (a * d).partial_cmp(&(b * c))
    }
}
