use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars::{self, JsonSchema};

#[cfg(feature = "astroport")]
pub use self::astroport::UsdcAxelar as Lpn;
#[cfg(feature = "osmosis-osmosis-usdc_axelar")]
pub use self::osmosis_osmosis_usdc_axelar::Usdc as Lpn;
#[cfg(feature = "osmosis-osmosis-usdc_noble")]
pub use self::osmosis_osmosis_usdc_noble::UsdcNoble as Lpn;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis-osmosis-usdc_axelar")]
mod osmosis_osmosis_usdc_axelar;
#[cfg(feature = "osmosis-osmosis-usdc_noble")]
mod osmosis_osmosis_usdc_noble;

#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Lpns {}

impl Group for Lpns {
    const DESCR: &'static str = "lpns";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        currency::maybe_visit_any::<_, Lpn, _>(matcher, symbol, visitor)
    }
}
