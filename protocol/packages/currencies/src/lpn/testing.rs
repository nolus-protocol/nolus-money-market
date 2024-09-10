use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, LeaseC4, Lpns, PaymentGroup};

define_currency!(Lpn, "LPN", "ibc/test_LPN", "ibc/test_DEX_LPN", Lpns, 6);

impl PairsGroup for Lpn {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::maybe_visit_buddy::<LeaseC4, _, _>(matcher, visitor)
    }
}
