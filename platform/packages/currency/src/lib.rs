#[cfg(all(feature = "impl", not(any(dex = "osmosis"))))]
compile_error!("No dex selected!");

#[cfg(all(feature = "impl", not(any(net = "dev", net = "test", net = "main"))))]
compile_error!("No net selected!");

use serde::{de::DeserializeOwned, Serialize};

mod currency;
pub use crate::currency::*;

#[cfg(feature = "impl")]
pub mod dex;

pub mod error;

mod nls;
pub use nls::{Native as NativePlatform, NlsPlatform};

#[cfg(any(test, feature = "testing"))]
pub mod test;

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
