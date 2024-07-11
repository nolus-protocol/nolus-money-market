use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult};
use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_AXELAR {
        // full ibc route: transfer/channel-0/transfer/channel-208/uusdc
        bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
        // full ibc route: transfer/channel-208/uusdc
        dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
    }
}
define_currency!(UsdcAxelar, USDC_AXELAR, 6);

pub(super) fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, UsdcAxelar, _>(matcher, visitor)
}
