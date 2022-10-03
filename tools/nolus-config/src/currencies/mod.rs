use serde::{Deserialize, Serialize};

use self::currency::Currency;
pub use self::generate::{
    CurrencyFilenameSource, CurrencySources, CurrencySourcesIter, GenerationResult,
    GroupFilenameSource, GroupSources, GroupsSourcesIter,
};

mod currency;
mod generate;
mod group;

#[derive(Serialize, Deserialize)]
pub struct Currencies {
    currencies: Vec<Currency>,
}
