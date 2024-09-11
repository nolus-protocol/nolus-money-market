use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::Ntrn, Native, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/6C9E6701AC217C0FC7D74B0F7A6265B9B4E3C3CDA6E80AADE5F950A8F52F9972", // transfer/channel-44/unls
    Native,
    6
);

impl PairsGroup for Nls {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Ntrn, _, _>(matcher, visitor)
    }
}
