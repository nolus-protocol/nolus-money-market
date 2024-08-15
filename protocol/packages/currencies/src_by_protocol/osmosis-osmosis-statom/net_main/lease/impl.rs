use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, LeaseGroup};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_currency!(
    UsdcNoble,
    "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4", // transfer/channel-0/transfer/channel-750/uusdc
    "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4", // transfer/channel-750/uusdc
    LeaseGroup,
    6
);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    LeaseGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, UsdcNoble, TopG, _>(matcher, visitor)
}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {
            lease::LeaseGroup,
            lpn::{Lpn, Lpns},
            native::Nls,
        },
    };

    use super::UsdcNoble;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcNoble, LeaseGroup>();

        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<UsdcNoble, LeaseGroup>(UsdcNoble::bank());
        maybe_visit_on_ticker_err::<UsdcNoble, LeaseGroup>(UsdcNoble::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcNoble, LeaseGroup>();

        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(UsdcNoble::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<UsdcNoble, LeaseGroup>(Nls::ticker());
    }
}
