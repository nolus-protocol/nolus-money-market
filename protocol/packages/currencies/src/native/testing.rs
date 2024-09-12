use currency::{InPoolWith, Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, Lpn, Native, PaymentC7, PaymentGroup};

define_currency!(Nls, "NLS", "unls", "ibc/test_DEX_NLS", Native, 6,);

impl PairsGroup for Nls {
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

impl InPoolWith<PaymentC7> for Nls {}
