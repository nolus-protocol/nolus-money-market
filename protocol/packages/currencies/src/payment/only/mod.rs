use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult};
use sdk::schemars::{self, JsonSchema};

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PaymentOnlyGroup {}

impl Group for PaymentOnlyGroup {
    const DESCR: &'static str = "payment only";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}
