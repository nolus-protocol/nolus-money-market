use sdk::schemars;

use currency::{
    AnyVisitor, Group, InPoolWith, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsGroup, PairsVisitor,
};

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
    StAtom,
    "ST_ATOM",
    "ibc/FCFF8B19C61677F3B78E2A5AE3B4A34A8D23858D16905F253B8438B3AFD07FF8", // transfer/channel-0/transfer/channel-326/stuatom
    "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901", // transfer/channel-326/stuatom
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

define_currency!(
    AllBtc,
    "ALL_BTC",
    "ibc/E45CFCB959F4F6D1065B7033EE49A88E606E6AD82E75725219B3D68B0FA89987", // transfer/channel-0/allBTC
    "factory/osmo1z6r6qdknhgsc0zeracktgpcxf43j6sekq07nw8sxduc9lg0qjjlqfu25e3/alloyed/allBTC",
    PaymentOnlyGroup,
    8
);

define_currency!(
    AllSol,
    "ALL_SOL",
    "ibc/762E1E45658845A12E214A91C3C05FDFC5951D60404FAADA225A369A96DCD9A9", // transfer/channel-0/allSOL
    "factory/osmo1n3n75av8awcnw4jl62n3l48e6e4sxqmaf97w5ua6ddu4s475q5qq9udvx4/alloyed/allSOL",
    PaymentOnlyGroup,
    9
);

define_currency!(
    AllEth,
    "ALL_ETH",
    "ibc/7879B1CBBD2E07347002334792368E65C11A7D1629297D04B6A2155F557E02D4", // transfer/channel-0/allETH
    "factory/osmo1k6c8jln7ejuqwtqmay3yvzrg3kueaczl96pk067ldg8u835w0yhsw27twm/alloyed/allETH",
    PaymentOnlyGroup,
    18
);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    PaymentOnlyGroup: MemberOf<TopG>,
    TopG: Group<TopG = PaymentGroup>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, Atom, TopG, _>(matcher, visitor)
        .or_else(|v| maybe_visit::<_, StAtom, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, Osmo, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, StOsmo, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, Weth, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, Inj, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, Axl, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, AllBtc, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, AllSol, TopG, _>(matcher, v))
        .or_else(|v| maybe_visit::<_, AllEth, TopG, _>(matcher, v))
}

impl PairsGroup for Atom {
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
impl InPoolWith<StAtom> for Atom {}
impl InPoolWith<UsdcNoble> for Atom {}

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

impl PairsGroup for Osmo {
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
impl InPoolWith<StOsmo> for Osmo {}
impl InPoolWith<Axl> for Osmo {}
impl InPoolWith<Weth> for Osmo {}
impl InPoolWith<Atom> for Osmo {}

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

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<UsdcNoble, _, _>(matcher, visitor)
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

impl PairsGroup for AllBtc {
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
impl InPoolWith<AllSol> for AllBtc {}

impl PairsGroup for AllSol {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<AllBtc, _, _>(matcher, visitor)
    }
}

impl PairsGroup for AllEth {
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

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::Lpn,
        payment::only::PaymentOnlyGroup,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{AllBtc, AllEth, AllSol, Atom, Axl, StAtom};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<AllSol, PaymentOnlyGroup>();
        maybe_visit_on_ticker_impl::<AllEth, PaymentOnlyGroup>();
        maybe_visit_on_ticker_err::<AllBtc, PaymentOnlyGroup>(Atom::bank());
        maybe_visit_on_ticker_err::<Axl, PaymentOnlyGroup>(Lpn::ticker());
        maybe_visit_on_ticker_err::<StAtom, PaymentOnlyGroup>(StAtom::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<AllSol, PaymentOnlyGroup>();
        maybe_visit_on_bank_symbol_impl::<AllEth, PaymentOnlyGroup>();
        maybe_visit_on_bank_symbol_err::<AllBtc, PaymentOnlyGroup>(Atom::ticker());
        maybe_visit_on_bank_symbol_err::<Axl, PaymentOnlyGroup>(Lpn::bank());
        maybe_visit_on_bank_symbol_err::<StAtom, PaymentOnlyGroup>(StAtom::dex());
    }
}
