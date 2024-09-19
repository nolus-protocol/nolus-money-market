use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, LeaseGroup, Lpn, Nls, PaymentGroup};

define_currency!(
    LeaseC1,
    "LC1",
    "ibc/B62610294777CD7D4567F7125B5D88DE95C6B7F7ED25430F3808F863202BC599", // transfer/channel-1/transfer/channel-1/uatom
    "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9", // transfer/channel-1/uatom
    LeaseGroup,
    6
);

define_currency!(
    LeaseC2,
    "LC2",
    "ibc/FFE21A5F04A89CD5E61A616EEE11A646D5DCF0A8DB60523F79C5ED28DA2642FA", // transfer/channel-3839/transfer/channel-8/stuatom
    "ibc/B7864B03E1B9FD4F049243E92ABD691586F682137037A9F3FCA5222815620B3C", // transfer/channel-8/stuatom
    LeaseGroup,
    6
);

define_currency!(
    LeaseC3,
    "LC3",
    "ibc/0C698C8970DB4C539455E5225665A804F6338753211319E44BAD39758B238695", // transfer/channel-1/untrn
    "untrn",
    LeaseGroup,
    6
);

define_currency!(
    LeaseC4,
    "LC4",
    "ibc/6DF8CF5C976851D152E2C7270B0AB25C4F9D64C0A46513A68D6CBB2662A98DF4", // transfer/channel-3839/transfer/channel-48/adydx
    "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130", // transfer/channel-48/adydx
    LeaseGroup,
    18
);

define_currency!(
    LeaseC5,
    "LC5",
    "ibc/8970C50B6F78D9AB5D0656E6BBD234BC1132ACBF680B8B6F506BB90CD2A06D81", // transfer/channel-3839/transfer/channel-35/utia
    "ibc/773B4D0A3CD667B2275D5A4A7A2F0909C0BA0F4059C0B9181E680DDF4965DCC7", // transfer/channel-35/utia
    LeaseGroup,
    6
);

define_currency!(
    LeaseC6,
    "LC6",
    "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221", // transfer/channel-0/transfer/channel-208/wbtc-satoshi
    "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F", // transfer/channel-208/wbtc-satoshi
    LeaseGroup,
    8
);

define_currency!(
    LeaseC7,
    "LC7",
    "ibc/2435225A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
    "ibc/C1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
    LeaseGroup,
    4
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
    maybe_visit::<_, LeaseC1, VisitedG, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, LeaseC2, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC3, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC4, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC5, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC6, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC7, VisitedG, _>(matcher, visitor))
}

impl PairsGroup for LeaseC1 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<LeaseC3, _, _>(matcher, visitor)
            .or_else(|v| maybe_visit::<LeaseC2, _, _>(matcher, v))
    }
}

impl PairsGroup for LeaseC2 {
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
impl InPoolWith<LeaseC1> for LeaseC2 {}
impl InPoolWith<LeaseC3> for LeaseC2 {}
impl InPoolWith<LeaseC4> for LeaseC2 {}

impl PairsGroup for LeaseC3 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<LeaseC2, _, _>(matcher, visitor)
    }
}
impl InPoolWith<LeaseC1> for LeaseC3 {}

impl PairsGroup for LeaseC4 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<LeaseC2, _, _>(matcher, visitor)
    }
}

impl PairsGroup for LeaseC5 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Nls, _, _>(matcher, visitor)
    }
}

impl PairsGroup for LeaseC6 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor) // let's stay detached from the swap tree for some corner cases
    }
}

impl PairsGroup for LeaseC7 {
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

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lease::LeaseGroup,
        lpn::Lpn,
        native::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        Lpns,
    };

    use super::{LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<LeaseC2, LeaseGroup>(LeaseC2::bank());
        maybe_visit_on_ticker_err::<LeaseC3, LeaseGroup>(LeaseC3::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::dex());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC5, LeaseGroup>(LeaseC5::ticker());
    }
}
