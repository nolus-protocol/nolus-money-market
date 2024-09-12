use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, Lpn, Native, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/E808FAAE7ADDA37453A8F0F67D74669F6580CBA5EF0F7889D46FB02D282098E3", // transfer/channel-1061/unls
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
