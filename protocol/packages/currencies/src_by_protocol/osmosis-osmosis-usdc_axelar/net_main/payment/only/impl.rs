use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, define_symbol, PaymentOnlyGroup};

define_symbol! {
    USDC_NOBLE {
        // full ibc route: transfer/channel-0/transfer/channel-750/uusdc
        bank: "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4",
        // full ibc route: transfer/channel-750/uusdc
        dex: "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4",
    }
}
define_currency!(UsdcNoble, USDC_NOBLE, PaymentOnlyGroup, 6);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher<Group = PaymentOnlyGroup>,
    V: AnyVisitor<TopG>,
    PaymentOnlyGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, UsdcNoble, TopG, _>(matcher, visitor)
}

#[cfg(test)]
mod test {
    use currency::Definition;

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
        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(Lpn::TICKER);
        maybe_visit_on_ticker_err::<Lpn, Lpns>(UsdcNoble::TICKER);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcNoble, PaymentOnlyGroup>();
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::TICKER);
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Lpn::BANK_SYMBOL);
    }
}
