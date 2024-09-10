use currency::{
    AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf, PairsGroup,
    PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, PaymentGroup, PaymentOnlyGroup};

define_currency!(
    UsdcNoble,
    "USDC_NOBLE",
    "ibc/83C68CA1189A7DAC4FDA8B89F9166FFA6BA3A8C5534B0E3D84D831B4F350FE59", // transfer/channel-0/transfer/channel-4280/uusdc
    "ibc/DE6792CF9E521F6AD6E9A4BDF6225C9571A3B74ACC0A529F92BC5122A39D2E58", // transfer/channel-4280/uusdc
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

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}

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
    }
}
