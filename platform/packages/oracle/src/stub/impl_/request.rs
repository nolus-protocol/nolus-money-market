use currency::{CurrencyDef, Group};
use serde::Serialize;

pub use self::base::BasePriceRequest;

mod base;

pub trait RequestBuilder {
    fn currency<G>() -> impl Serialize
    where
        G: Group;

    fn price<C>() -> impl Serialize
    where
        C: CurrencyDef;
}
