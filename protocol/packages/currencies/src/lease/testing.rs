use currency::{
    AnyVisitor, CurrencyDef, Group, InPoolWith, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, LeaseGroup, PaymentGroup};

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

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    LeaseGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, LeaseC1, TopG, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, LeaseC2, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC3, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC4, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, LeaseC5, TopG, _>(matcher, visitor))
}

pub(crate) fn maybe_visit_buddy<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
    M: Matcher,
    V: PairsVisitor<Pivot = PaymentGroup, VisitedG = PaymentGroup>,
{
    use currency::maybe_visit_buddy as maybe_visit;
    maybe_visit::<LeaseC1, _, _>(LeaseC1::definition().dto(), matcher, visitor)
        .or_else(|visitor| {
            maybe_visit::<LeaseC2, _, _>(LeaseC2::definition().dto(), matcher, visitor)
        })
        .or_else(|visitor| {
            maybe_visit::<LeaseC3, _, _>(LeaseC3::definition().dto(), matcher, visitor)
        })
        .or_else(|visitor| {
            maybe_visit::<LeaseC4, _, _>(LeaseC4::definition().dto(), matcher, visitor)
        })
        .or_else(|visitor| {
            maybe_visit::<LeaseC5, _, _>(LeaseC5::definition().dto(), matcher, visitor)
        })
}

impl InPoolWith<PaymentGroup> for LeaseC1 {}
impl PairsGroup for LeaseC1 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}

impl InPoolWith<PaymentGroup> for LeaseC2 {}
impl PairsGroup for LeaseC2 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}

impl InPoolWith<PaymentGroup> for LeaseC3 {}
impl PairsGroup for LeaseC3 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}

impl InPoolWith<PaymentGroup> for LeaseC4 {}
impl PairsGroup for LeaseC4 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}

impl InPoolWith<PaymentGroup> for LeaseC5 {}
impl PairsGroup for LeaseC5 {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
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
