use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::Dydx, Lpn, PaymentGroup, PaymentOnlyGroup};

define_currency!(
    UsdcNoble,
    "USDC_NOBLE",
    "ibc/18161D8EFBD00FF5B7683EF8E923B8913453567FBE3FB6672D75712B0DEB6682", // transfer/channel-3839/transfer/channel-30/uusdc
    "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81", // transfer/channel-30/uusdc
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
    maybe_visit::<_, UsdcNoble, VisitedG, _>(matcher, visitor)
}

impl PairsGroup for UsdcNoble {
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
impl InPoolWith<Dydx> for UsdcNoble {}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::{Lpn, Lpns},
        native::Nls,
        payment::only::PaymentOnlyGroup,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::UsdcNoble;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcNoble, PaymentOnlyGroup>();

        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::bank());
        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(Lpn::ticker());
        maybe_visit_on_ticker_err::<Lpn, Lpns>(UsdcNoble::ticker());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcNoble, PaymentOnlyGroup>();

        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Lpn::bank());
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(UsdcNoble::bank());
    }
}
