use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars::{self, JsonSchema};

#[cfg(any(
    feature = "neutron-astroport-usdc_axelar",
    feature = "neutron-astroport-usdc_noble"
))]
use self::astroport as impl_mod;
#[cfg(any(
    feature = "osmosis-osmosis-usdc_axelar",
    feature = "osmosis-osmosis-usdc_noble"
))]
use self::osmosis as impl_mod;

#[cfg(any(
    feature = "neutron-astroport-usdc_axelar",
    feature = "neutron-astroport-usdc_noble"
))]
pub(crate) mod astroport;
#[cfg(any(
    feature = "osmosis-osmosis-usdc_axelar",
    feature = "osmosis-osmosis-usdc_noble"
))]
pub(crate) mod osmosis;

#[derive(Clone, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        impl_mod::maybe_visit(matcher, symbol, visitor)
    }
}
