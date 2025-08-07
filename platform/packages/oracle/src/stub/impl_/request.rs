use currency::{CurrencyDef, Group};
use serde::Serialize;

pub use self::base::BasePriceRequest;

#[cfg(feature = "unchecked-stable-quote")]
pub use self::stable::StablePriceRequest;

mod base;
#[cfg(feature = "unchecked-stable-quote")]
mod stable;

pub trait RequestBuilder {
    fn currency<G>() -> impl Serialize
    where
        G: Group;

    fn price<C>() -> impl Serialize
    where
        C: CurrencyDef;
}
