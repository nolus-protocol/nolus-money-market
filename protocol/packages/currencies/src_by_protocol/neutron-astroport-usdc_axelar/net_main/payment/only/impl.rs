use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, define_symbol, PaymentOnlyGroup};

define_symbol! {
    USDC_NOBLE {
        // full ibc route: transfer/channel-3839/transfer/channel-30/uusdc
        bank: "ibc/18161D8EFBD00FF5B7683EF8E923B8913453567FBE3FB6672D75712B0DEB6682",
        // full ibc route: transfer/channel-30/uusdc
        dex: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81",
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
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(UsdcNoble::BANK_SYMBOL);
    }
}
