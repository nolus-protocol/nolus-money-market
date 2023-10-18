#[cfg(feature = "impl")]
use serde::{de::DeserializeOwned, Serialize};

mod currency;
pub use crate::currency::*;

#[cfg(feature = "impl")]
mod currency_macro;

pub mod error;

#[cfg(feature = "impl")]
pub mod lease;

#[cfg(feature = "impl")]
pub mod lpn;

#[cfg(feature = "impl")]
pub mod native;

mod nls;
pub use nls::NlsPlatform;

#[cfg(feature = "impl")]
pub mod payment;

#[cfg(feature = "impl")]
mod symbols_macro;

#[cfg(any(test, feature = "testing"))]
pub mod test;

#[cfg(feature = "impl")]
fn maybe_visit_any<M, C, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    C: Currency + Serialize + DeserializeOwned,
    V: AnyVisitor,
{
    if matcher.match_::<C>(symbol) {
        Ok(visitor.on::<C>())
    } else {
        Err(visitor)
    }
}
