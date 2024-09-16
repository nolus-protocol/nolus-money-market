use currency::{
    AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf, PairsGroup,
    PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, Lpn, PaymentGroup, PaymentOnlyGroup};

define_currency!(
    Atom,
    "ATOM",
    "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388", // transfer/channel-0/transfer/channel-0/uatom
    "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", // transfer/channel-0/uatom
    PaymentOnlyGroup,
    6
);

pub(super) fn maybe_visit<M, V, VisitedG>(
    matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    PaymentOnlyGroup: MemberOf<VisitedG>,
    VisitedG: Group<TopG = PaymentGroup>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, Atom, VisitedG, _>(matcher, visitor)
}

impl PairsGroup for Atom {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Lpn, _, _>(matcher, visitor)
    }
}
