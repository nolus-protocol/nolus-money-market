use sdk::schemars;

use currency::{PairsGroup, PairsVisitor, MaybePairsVisitorResult, Matcher};

use crate::{define_currency, Native, Lpn, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/EF145240FE393A1CEC9C35ED1866A235D23176EA9B32069F714C9309FEA55718", // transfer/channel-8272/unls
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
        maybe_visit::<Lpn, _, _>(matcher, visitor)
    }
}
