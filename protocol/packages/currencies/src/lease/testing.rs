use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    LC1 {
        // full ibc route: transfer/channel-1/transfer/channel-1/uatom
        bank: "ibc/B62610294777CD7D4567F7125B5D88DE95C6B7F7ED25430F3808F863202BC599",
        // full ibc route: transfer/channel-1/uatom
        dex: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
    }
}
define_currency!(LeaseC1, LC1, 6);

define_symbol! {
    LC2 {
        // full ibc route: transfer/channel-3839/transfer/channel-8/stuatom
        bank: "ibc/FFE21A5F04A89CD5E61A616EEE11A646D5DCF0A8DB60523F79C5ED28DA2642FA",
        // full ibc route: transfer/channel-8/stuatom
        dex: "ibc/B7864B03E1B9FD4F049243E92ABD691586F682137037A9F3FCA5222815620B3C",
    }
}
define_currency!(LeaseC2, LC2, 6);

define_symbol! {
    LC3 {
        // full ibc route: transfer/channel-1/untrn
        bank: "ibc/0C698C8970DB4C539455E5225665A804F6338753211319E44BAD39758B238695",
        dex: "untrn",
    }
}
define_currency!(LeaseC3, LC3, 6);

define_symbol! {
    LC4 {
        // full ibc route: transfer/channel-3839/transfer/channel-48/adydx
        bank: "ibc/6DF8CF5C976851D152E2C7270B0AB25C4F9D64C0A46513A68D6CBB2662A98DF4",
        // full ibc route: transfer/channel-48/adydx
        dex: "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130",
    }
}
define_currency!(LeaseC4, LC4, 18);

define_symbol! {
    LC5 {
        // full ibc route: transfer/channel-3839/transfer/channel-35/utia
        bank: "ibc/8970C50B6F78D9AB5D0656E6BBD234BC1132ACBF680B8B6F506BB90CD2A06D81",
        // full ibc route: transfer/channel-35/utia
        dex: "ibc/773B4D0A3CD667B2275D5A4A7A2F0909C0BA0F4059C0B9181E680DDF4965DCC7",
    }
}
define_currency!(LeaseC5, LC5, 6);

pub(super) fn maybe_visit<M, V>(
    matcher: &M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, LeaseC1, _>(matcher, symbol, visitor)
        .or_else(|visitor| maybe_visit::<_, LeaseC2, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC3, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC4, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC5, _>(matcher, symbol, visitor))
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {lease::LeaseGroup, lpn::Lpn, native::Nls},
    };

    use super::{LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_ticker_err::<Lpn, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Lpn, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<LeaseC2, LeaseGroup>(LeaseC2::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<LeaseC3, LeaseGroup>(LeaseC3::DEX_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, LeaseGroup>(Lpn::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::TICKER);
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::DEX_SYMBOL);
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<LeaseC5, LeaseGroup>(LeaseC5::TICKER);
    }
}