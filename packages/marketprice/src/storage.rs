use finance::currency::SymbolOwned;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::price::PriceDTO;
use finance::{
    coin::Coin as FinCoin, currency::Currency, fraction::Fraction, fractionable::HigherRank,
    ratio::Rational,
};

use crate::market_price::PriceFeedsError;

#[deprecated = "Migrate to using finance::coin::Coin"]
pub type DenomPair = (SymbolOwned, SymbolOwned);
