use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{
    define_currency, payment::only::impl_mod::UsdcNoble, LeaseGroup, Lpn, Nls, PaymentGroup,
};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_currency!(
    Atom,
    "ATOM",
    "ibc/74329D0B6EAA37AD07FF44EC40D998357D1478C504AB5A9C91C3F42F1078A226", // transfer/channel-3839/transfer/channel-1/uatom
    "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9", // transfer/channel-1/uatom
    LeaseGroup,
    6
);

define_currency!(
    StAtom,
    "ST_ATOM",
    "ibc/FFE21A5F04A89CD5E61A616EEE11A646D5DCF0A8DB60523F79C5ED28DA2642FA", // transfer/channel-3839/transfer/channel-8/stuatom
    "ibc/B7864B03E1B9FD4F049243E92ABD691586F682137037A9F3FCA5222815620B3C", // transfer/channel-8/stuatom
    LeaseGroup,
    6
);

define_currency!(
    Ntrn,
    "NTRN",
    "ibc/3D6BC6E049CAEB905AC97031A42800588C58FB471EBDC7A3530FFCD0C3DC9E09", // transfer/channel-3839/untrn
    "untrn",
    LeaseGroup,
    6
);

define_currency!(
    Dydx,
    "DYDX",
    "ibc/6DF8CF5C976851D152E2C7270B0AB25C4F9D64C0A46513A68D6CBB2662A98DF4", // transfer/channel-3839/transfer/channel-48/adydx
    "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130", // transfer/channel-48/adydx
    LeaseGroup,
    18
);

define_currency!(
    Tia,
    "TIA",
    "ibc/8970C50B6F78D9AB5D0656E6BBD234BC1132ACBF680B8B6F506BB90CD2A06D81", // transfer/channel-3839/transfer/channel-35/utia
    "ibc/773B4D0A3CD667B2275D5A4A7A2F0909C0BA0F4059C0B9181E680DDF4965DCC7", // transfer/channel-35/utia
    LeaseGroup,
    6
);

define_currency!(
    StTia,
    "ST_TIA",
    "ibc/13B5CDC63B4A997453AF882CFC67BFDF932910C1BF3795C34B89548F2C5B1CD1", // transfer/channel-3839/transfer/channel-8/stutia
    "ibc/6569E05DEE32B339D9286A52BE33DFCEFC97267F23EF9CFDE0C055140967A9A5", // transfer/channel-8/stutia
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
    maybe_visit::<_, Atom, VisitedG, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, StAtom, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Ntrn, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Dydx, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Tia, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, StTia, VisitedG, _>(matcher, visitor))
}

impl PairsGroup for Atom {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Ntrn, _, _>(matcher, visitor)
    }
}
impl InPoolWith<StAtom> for Atom {}

impl PairsGroup for StAtom {
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

impl PairsGroup for Ntrn {
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
impl InPoolWith<Atom> for Ntrn {}
impl InPoolWith<Nls> for Ntrn {}
impl InPoolWith<Tia> for Ntrn {}

impl PairsGroup for Dydx {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<UsdcNoble, _, _>(matcher, visitor)
    }
}

impl PairsGroup for Tia {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Ntrn, _, _>(matcher, visitor)
    }
}
impl InPoolWith<StTia> for Tia {}

impl PairsGroup for StTia {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Tia, _, _>(matcher, visitor)
    }
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

    use super::{Atom, Dydx, Ntrn, StAtom, StTia, Tia};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Ntrn, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Dydx, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Tia, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StTia, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::dex());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::ticker());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Lpn::bank());
        maybe_visit_on_ticker_err::<Dydx, LeaseGroup>(Dydx::bank());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Dydx, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Tia, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StTia, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::dex());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::ticker());
        maybe_visit_on_bank_symbol_err::<Dydx, LeaseGroup>(Dydx::ticker());
    }
}
