use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::UsdcNoble, Lpn, PaymentGroup, PaymentOnlyGroup};

define_currency!(
    Atom,
    "ATOM",
    "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388", // transfer/channel-0/transfer/channel-0/uatom
    "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", // transfer/channel-0/uatom
    PaymentOnlyGroup,
    6
);

define_currency!(
    Osmo,
    "OSMO",
    "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518", // transfer/channel-0/uosmo
    "uosmo",
    PaymentOnlyGroup,
    6
);

define_currency!(
    StOsmo,
    "ST_OSMO",
    "ibc/AF5559D128329B6C753F15481BEC26E533B847A471074703FA4903E7E6F61BA1", // transfer/channel-0/transfer/channel-326/stuosmo
    "ibc/D176154B0C63D1F9C6DCFB4F70349EBF2E2B5A87A05902F57A6AE92B863E9AEC", // transfer/channel-326/stuosmo
    PaymentOnlyGroup,
    6
);

define_currency!(
    Weth,
    "WETH",
    "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C", // transfer/channel-0/transfer/channel-208/weth-wei
    "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5", // transfer/channel-208/weth-wei
    PaymentOnlyGroup,
    18
);

define_currency!(
    Wbtc,
    "WBTC",
    "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221", // transfer/channel-0/transfer/channel-208/wbtc-satoshi
    "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F", // transfer/channel-208/wbtc-satoshi
    PaymentOnlyGroup,
    8
);

define_currency!(
    Akt,
    "AKT",
    "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459", // transfer/channel-0/transfer/channel-1/uakt
    "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4", // transfer/channel-1/uakt
    PaymentOnlyGroup,
    6
);

define_currency!(
    Inj,
    "INJ",
    "ibc/4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E", // transfer/channel-0/transfer/channel-122/inj
    "ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273", // transfer/channel-122/inj
    PaymentOnlyGroup,
    18
);

define_currency!(
    Axl,
    "AXL",
    "ibc/1B03A71B8E6F6EF424411DC9326A8E0D25D096E4D2616425CFAF2AF06F0FE717", // transfer/channel-0/transfer/channel-208/uaxl
    "ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E", // transfer/channel-208/uaxl
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
    maybe_visit::<_, Atom, VisitedG, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, Osmo, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, StOsmo, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Inj, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Axl, VisitedG, _>(matcher, visitor))
}

impl PairsGroup for Atom {
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
impl InPoolWith<UsdcNoble> for Atom {}
impl InPoolWith<Osmo> for Atom {}

impl PairsGroup for Osmo {
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
impl InPoolWith<StOsmo> for Osmo {}
impl InPoolWith<Akt> for Osmo {}
impl InPoolWith<Axl> for Osmo {}

impl PairsGroup for StOsmo {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Osmo, _, _>(matcher, visitor)
    }
}

impl PairsGroup for Weth {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}

impl PairsGroup for Wbtc {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}

impl PairsGroup for Akt {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Osmo, _, _>(matcher, visitor)
    }
}

impl PairsGroup for Inj {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}

impl PairsGroup for Axl {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<Osmo, _, _>(matcher, visitor)
    }
}
