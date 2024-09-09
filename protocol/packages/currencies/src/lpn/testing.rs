use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, Lpns, PaymentGroup};

define_currency!(Lpn, "LPN", "ibc/test_LPN", "ibc/test_DEX_LPN", Lpns, 6);

impl PairsGroup for Lpn {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}
