use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, define_symbol, PaymentOnlyGroup};

ATOM {
    // full ibc route: transfer/channel-0/transfer/channel-0/uatom
    bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
    // full ibc route: transfer/channel-0/uatom
    dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
},
OSMO {
    // full ibc route: transfer/channel-0/uosmo
    bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
    dex: "uosmo",
},
ST_OSMO {
    // full ibc route: transfer/channel-0/transfer/channel-326/stuosmo
    bank: "ibc/AF5559D128329B6C753F15481BEC26E533B847A471074703FA4903E7E6F61BA1",
    // full ibc route: transfer/channel-326/stuosmo
    dex: "ibc/D176154B0C63D1F9C6DCFB4F70349EBF2E2B5A87A05902F57A6AE92B863E9AEC",
},
WETH {
    // full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
    bank: "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C",
    // full ibc route: transfer/channel-208/weth-wei
    dex: "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5",
},
WBTC {
    // full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
    bank: "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
    // full ibc route: transfer/channel-208/wbtc-satoshi
    dex: "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
},
AKT {
    // full ibc route: transfer/channel-0/transfer/channel-1/uakt
    bank: "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459",
    // full ibc route: transfer/channel-1/uakt
    dex: "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4",
},
INJ {
    // full ibc route: transfer/channel-0/transfer/channel-122/inj
    bank: "ibc/4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E",
    // full ibc route: transfer/channel-122/inj
    dex: "ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273",
},
AXL {
    // full ibc route: transfer/channel-0/transfer/channel-208/uaxl
    bank: "ibc/1B03A71B8E6F6EF424411DC9326A8E0D25D096E4D2616425CFAF2AF06F0FE717",
    // full ibc route: transfer/channel-208/uaxl
    dex: "ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E",
},
USDC_NOBLE {
    // full ibc route: transfer/channel-0/transfer/channel-750/uusdc
    bank: "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4",
    // full ibc route: transfer/channel-750/uusdc
    dex: "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4",
}

define_symbol! {
    USDC_AXELAR {
        // full ibc route: transfer/channel-0/transfer/channel-208/uusdc
        bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
        // full ibc route: transfer/channel-208/uusdc
        dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
    }
}
define_currency!(UsdcAxelar, USDC_AXELAR, PaymentOnlyGroup, 6);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    PaymentOnlyGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, UsdcAxelar, TopG, _>(matcher, visitor)
}
