use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{
    define_currency,
    payment::only::impl_mod::{AllBtc, AllEth, Atom, Inj},
    LeaseGroup, Nls, PaymentGroup,
};

define_currency!(
    UsdcNoble,
    "USDC_NOBLE",
    "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4", // transfer/channel-0/transfer/channel-750/uusdc
    "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4", // transfer/channel-750/uusdc
    LeaseGroup,
    6
);

pub(super) fn maybe_visit<M, V, VisitedG>(
    matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    LeaseGroup: MemberOf<VisitedG>,
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
        maybe_visit::<Atom, _, _>(matcher, visitor)
    }
}
impl InPoolWith<Nls> for UsdcNoble {}
impl InPoolWith<AllEth> for UsdcNoble {}
impl InPoolWith<Inj> for UsdcNoble {}
impl InPoolWith<AllBtc> for UsdcNoble {}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lease::LeaseGroup,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::UsdcNoble;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcNoble, LeaseGroup>();
        maybe_visit_on_ticker_err::<UsdcNoble, LeaseGroup>(UsdcNoble::bank());
        maybe_visit_on_ticker_err::<UsdcNoble, LeaseGroup>(UsdcNoble::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcNoble, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(UsdcNoble::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(UsdcNoble::dex());
    }
}
