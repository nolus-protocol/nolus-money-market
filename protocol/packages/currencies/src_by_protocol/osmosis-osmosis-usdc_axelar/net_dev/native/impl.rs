use sdk::schemars;

use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};

use crate::{define_currency, Native, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/48D5F90242DD5B460E139E1CCB503B0F7E44625CE7566BE74644F4600F5B5218", // transfer/channel-5733/unls
    Native,
    6
);

impl PairsGroup for Nls {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}
