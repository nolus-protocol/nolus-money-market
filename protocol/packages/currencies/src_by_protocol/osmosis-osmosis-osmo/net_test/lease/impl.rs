use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, LeaseGroup, Lpn, Nls, PaymentGroup};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_currency!(
    UsdcAxelar,
    "USDC_AXELAR",
    "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11", // transfer/channel-0/transfer/channel-3/uausdc
    "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE", // transfer/channel-3/uausdc
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
    maybe_visit::<_, UsdcAxelar, VisitedG, _>(matcher, visitor)
}

impl PairsGroup for UsdcAxelar {
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
impl InPoolWith<Nls> for UsdcAxelar {}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {
            lease::{impl_mod::UsdcAxelar, LeaseGroup},
            lpn::{Lpn, Lpns},
            native::Nls,
        },
    };

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcAxelar, LeaseGroup>();

        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<UsdcAxelar, LeaseGroup>(UsdcAxelar::bank());
        maybe_visit_on_ticker_err::<UsdcAxelar, LeaseGroup>(UsdcAxelar::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcAxelar, LeaseGroup>();

        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, LeaseGroup>(UsdcAxelar::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, LeaseGroup>(Nls::ticker());
    }
}
