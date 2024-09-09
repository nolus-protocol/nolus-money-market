use currency::{
    AnyVisitor, CurrencyDef, Group, InPoolWith, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, PaymentGroup, PaymentOnlyGroup};

define_currency!(
    UsdcAxelar,
    "USDC_AXELAR",
    "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911", // transfer/channel-0/transfer/channel-208/uusdc
    "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858", // transfer/channel-208/uusdc
    PaymentOnlyGroup,
    6
);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    PaymentOnlyGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, UsdcAxelar, TopG, _>(matcher, visitor)
}

pub(crate) fn maybe_visit_buddy<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
    M: Matcher,
    V: PairsVisitor<Pivot = PaymentGroup, VisitedG = PaymentGroup>,
{
    use currency::maybe_visit_buddy as maybe_visit;
    maybe_visit::<UsdcAxelar, _, _>(UsdcAxelar::definition().dto(), matcher, visitor)
}

impl InPoolWith<PaymentGroup> for UsdcAxelar {}

impl PairsGroup for UsdcAxelar {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}
