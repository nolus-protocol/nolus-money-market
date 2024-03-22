use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{define_currency, define_symbol};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

#[cfg(feature = "osmosis-osmosis-usdc_axelar")]
define_symbol! {
    ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-12/uatom
            bank: "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196",
            // full ibc route: transfer/channel-12/uatom
            dex: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-12/uatom
            bank: "ibc/CFAC783D503ABF2BD3C9BB1D2AC6CD6136192782EE936D9BE406977F6D133926",
            // full ibc route: transfer/channel-12/uatom
            dex: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-0/uatom
            bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
            // full ibc route: transfer/channel-0/uatom
            dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        },
    }
}
#[cfg(feature = "osmosis-osmosis-usdc_noble")]
define_symbol! {
    ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-12/uatom
            bank: "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196",
            // full ibc route: transfer/channel-12/uatom
            dex: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-4156/uatom
            bank: "ibc/31104FCE0412CA93333DC76017D723CD3995866662B1C45A269EED8F05B378EB",
            // full ibc route: transfer/channel-4156/uatom
            dex: "ibc/9FF2B7A5F55038A7EE61F4FD6749D9A648B48E89830F2682B67B5DC158E2753C",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-0/uatom
            bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
            // full ibc route: transfer/channel-0/uatom
            dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        },
    }
}
define_currency!(Atom, ATOM, 6);

define_symbol! {
    ST_ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/uatom
            bank: "ibc/NA_ST_ATOM",
            // full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_ATOM_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/uatom
            bank: "ibc/NA_ST_ATOM",
            // full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_ATOM_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-326/stuatom
            bank: "ibc/FCFF8B19C61677F3B78E2A5AE3B4A34A8D23858D16905F253B8438B3AFD07FF8",
            // full ibc route: transfer/channel-326/stuatom
            dex: "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901",
        },
    }
}
define_currency!(StAtom, ST_ATOM, 6);

define_symbol! {
    OSMO {
        ["net_dev", "net_main"]: {
            // full ibc route: transfer/channel-0/uosmo
            bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
            dex: "uosmo",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/uosmo
            bank: "ibc/0A9CB406B20A767719CDA5C36D3F9939C529B96D122E7B42C09B9BA1F8E84298",
            dex: "uosmo",
        },
    }
}
define_currency!(Osmo, OSMO, 6);

define_symbol! {
    ST_OSMO {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/uatom
            bank: "ibc/NA_ST_OSMO",
            // full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_OSMO_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/uatom
            bank: "ibc/NA_ST_OSMO",
            // full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_OSMO_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-326/stuosmo
            bank: "ibc/AF5559D128329B6C753F15481BEC26E533B847A471074703FA4903E7E6F61BA1",
            // full ibc route: transfer/channel-326/stuosmo
            dex: "ibc/D176154B0C63D1F9C6DCFB4F70349EBF2E2B5A87A05902F57A6AE92B863E9AEC",
        },
    }
}
define_currency!(StOsmo, ST_OSMO, 6);

define_symbol! {
    WETH {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-3/eth-wei
            bank: "ibc/98CD37B180F06F954AFC71804049BE6EEA2A3B0CCEA1F425D141245BCFFBBD33",
            // full ibc route: transfer/channel-3/eth-wei
            // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            // although there is no pool WETH participates in
            dex: "ibc/29320BE25C3BF64A2355344625410899C1EB164038E328531C36095B0AA8BBFC",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-3/eth-wei
            bank: "ibc/B4353D6D9813CB7F3C540D3E99F48D99CD18A38D664A5E80DF738D7698AE4687",
            // full ibc route: transfer/channel-3/eth-wei
            // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            // although there is no pool WETH participates in
            dex: "ibc/29320BE25C3BF64A2355344625410899C1EB164038E328531C36095B0AA8BBFC",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
            bank: "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C",
            // full ibc route: transfer/channel-208/weth-wei
            dex: "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5",
        },
    }
}
define_currency!(Weth, WETH, 18);

define_symbol! {
    WBTC {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-3/btc-satoshi
            bank: "ibc/680E95D3CEA378B7302926B8A5892442F1F7DF78E22199AE248DCBADC9A0C1A2",
            // full ibc route: transfer/channel-3/btc-satoshi
            // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            // although there is no denomination trace as per `osmosisd q ibc-transfer denom-trace`
            dex: "ibc/CEDA3AFF171E72ACB689B7B64E988C0077DA7D4BF157637FFBDEB688D205A473",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-3/btc-satoshi
            bank: "ibc/DEFD6565A2C62E54CFB95562E29583D36E4C23ECADB130672A703366124ADD45",
            // full ibc route: transfer/channel-3/btc-satoshi
            // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            // although there is no denomination trace as per `osmosisd q ibc-transfer denom-trace`
            dex: "ibc/CEDA3AFF171E72ACB689B7B64E988C0077DA7D4BF157637FFBDEB688D205A473",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
            bank: "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
            // full ibc route: transfer/channel-208/wbtc-satoshi
            dex: "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
        },
    }
}
define_currency!(Wbtc, WBTC, 8);

define_symbol! {
    AKT {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-73/uakt
            bank: "ibc/1064EED4A8E99F9C1158680236D0C5C3EA6B8BB65C9F87DAC6BC759DD904D818",
            // full ibc route: transfer/channel-73/uakt
            dex: "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-73/uakt
            bank: "ibc/E3477DEE69A2AFF7A1665C2961C210132DD50954EF0AE171086189257FFC844F",
            // full ibc route: transfer/channel-73/uakt
            dex: "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-1/uakt
            bank: "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459",
            // full ibc route: transfer/channel-1/uakt
            dex: "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4",
        },
    }
}
define_currency!(Akt, AKT, 6);

define_symbol! {
    AXL {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-3/uaxl
            // not in use due to the lack of a pool
            bank: "ibc/NA_AXL",
            // full ibc route: transfer/channel-3/uaxl
            // not in use due to the lack of a pool
            dex: "ibc/NA_AXL_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-3/uaxl
            // not in use due to the lack of a pool
            bank: "ibc/NA_AXL",
            // full ibc route: transfer/channel-3/uaxl
            // not in use due to the lack of a pool
            dex: "ibc/NA_AXL_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-208/uaxl
            bank: "ibc/1B03A71B8E6F6EF424411DC9326A8E0D25D096E4D2616425CFAF2AF06F0FE717",
            // full ibc route: transfer/channel-208/uaxl
            dex: "ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E",
        },
    }
}
define_currency!(Axl, AXL, 6);

define_symbol! {
    Q_ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/uqatom
            bank: "ibc/NA_Q_ATOM",
            // full ibc route: transfer/channel-??/uqatom
            dex: "ibc/NA_Q_ATOM_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/uqatom
            bank: "ibc/NA_Q_ATOM",
            // full ibc route: transfer/channel-??/uqatom
            dex: "ibc/NA_Q_ATOM_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-522/uqatom
            bank: "ibc/317FCA2D7554F55BBCD0019AB36F7FEA18B6D161F462AF5E565068C719A29F20",
            // full ibc route: transfer/channel-522/uqatom
            dex: "ibc/FA602364BEC305A696CBDF987058E99D8B479F0318E47314C49173E8838C5BAC",
        },
    }
}
define_currency!(QAtom, Q_ATOM, 6);

define_symbol! {
    STK_ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/stk/uatom
            bank: "ibc/NA_STK_ATOM",
            // full ibc route: transfer/channel-??/stk/uatom
            dex: "ibc/NA_STK_ATOM_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/stk/uatom
            bank: "ibc/NA_STK_ATOM",
            // full ibc route: transfer/channel-??/stk/uatom
            dex: "ibc/NA_STK_ATOM_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-4/stk/uatom
            bank: "ibc/DAAD372DB7DD45BBCFA4DDD40CA9793E9D265D1530083AB41A8A0C53C3EBE865",
            // full ibc route: transfer/channel-4/stk/uatom
            dex: "ibc/CAA179E40F0266B0B29FB5EAA288FB9212E628822265D4141EBD1C47C3CBFCBC",
        },
    }
}
define_currency!(StkAtom, STK_ATOM, 6);

define_symbol! {
    STRD {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/ustrd
            bank: "ibc/NA_STRD",
            // full ibc route: transfer/channel-??/ustrd
            dex: "ibc/NA_STRD_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/ustrd
            bank: "ibc/NA_STRD",
            // full ibc route: transfer/channel-??/ustrd
            dex: "ibc/NA_STRD_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-326/ustrd
            bank: "ibc/04CA9067228BB51F1C39A506DA00DF07E1496D8308DD21E8EF66AD6169FA722B",
            // full ibc route: transfer/channel-326/ustrd
            dex: "ibc/A8CA5EE328FA10C9519DF6057DA1F69682D28F7D0F5CCC7ECB72E3DCA2D157A4",
        },
    }
}
define_currency!(Strd, STRD, 6);

define_symbol! {
    INJ {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/inj
            bank: "ibc/NA_INJ",
            // full ibc route: transfer/channel-??/inj
            dex: "ibc/NA_INJ_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/inj
            bank: "ibc/NA_INJ",
            // full ibc route: transfer/channel-??/inj
            dex: "ibc/NA_INJ_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-122/inj
            bank: "ibc/4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E",
            // full ibc route: transfer/channel-122/inj
            dex: "ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273",
        },
    }
}
define_currency!(Inj, INJ, 18);

define_symbol! {
    SCRT {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/uscrt
            bank: "ibc/NA_SCRT",
            // full ibc route: transfer/channel-??/uscrt
            dex: "ibc/NA_SCRT_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/uscrt
            bank: "ibc/NA_SCRT",
            // full ibc route: transfer/channel-??/uscrt
            dex: "ibc/NA_SCRT_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-88/uscrt
            bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
            // full ibc route: transfer/channel-88/uscrt
            dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
        },
    }
}
define_currency!(Secret, SCRT, 6);

define_symbol! {
    STARS {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/ustars
            bank: "ibc/NA_STARS",
            // full ibc route: transfer/channel-??/ustars
            dex: "ibc/NA_STARS_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/ustars
            bank: "ibc/NA_STARS",
            // full ibc route: transfer/channel-??/ustars
            dex: "ibc/NA_STARS_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-75/ustars
            bank: "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5",
            // full ibc route: transfer/channel-75/ustars
            dex: "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
        },
    }
}
define_currency!(Stars, STARS, 6);

define_symbol! {
    CRO {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/basecro
            bank: "ibc/NA_CRO",
            // full ibc route: transfer/channel-??/basecro
            dex: "ibc/NA_CRO_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/basecro
            bank: "ibc/NA_CRO",
            // full ibc route: transfer/channel-??/basecro
            dex: "ibc/NA_CRO_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-5/basecro
            bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
            // full ibc route: transfer/channel-5/basecro
            dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
        },
    }
}
define_currency!(Cro, CRO, 8);

define_symbol! {
    JUNO {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-1/ujunox
            bank: "ibc/8FB044422997A8A77891DE729EC28638DDE4C81A54398F68149A058AA9B74D9F",
            // full ibc route: transfer/channel-1/ujunox
            dex: "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-1/ujunox
            bank: "ibc/BEDEB6912C720F66B74F44620EA7A5C415E5BD0E78198ACEBF667D5974761835",
            // full ibc route: transfer/channel-1/ujunox
            dex: "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-42/ujuno
            bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
            // full ibc route: transfer/channel-42/ujuno
            dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        },
    }
}
define_currency!(Juno, JUNO, 6);

define_symbol! {
    EVMOS {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-??/aevmos
            bank: "ibc/NA_EVMOS",
            // full ibc route: transfer/channel-??/aevmos
            dex: "ibc/NA_EVMOS_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-??/aevmos
            bank: "ibc/NA_EVMOS",
            // full ibc route: transfer/channel-??/aevmos
            dex: "ibc/NA_EVMOS_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-204/aevmos
            bank: "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE",
            // full ibc route: transfer/channel-204/aevmos
            dex: "ibc/6AE98883D4D5D5FF9E50D7130F1305DA2FFA0C652D1DD9C123657C6B4EB2DF8A",
        },
    }
}
define_currency!(Evmos, EVMOS, 18);

define_symbol! {
    MARS {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-24/umars
            bank: "ibc/1CC042AD599E184C0F77DC5D89443C82F8A16B6E13DEC650A7A50A5D0AA330C3",
            // full ibc route: transfer/channel-24/umars
            dex: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-24/umars
            bank: "ibc/70B19E9BD830FC82B26C6E93B3A73D1D91EF3B01E5EF462EC371A3F84FB24944",
            // full ibc route: transfer/channel-24/umars
            dex: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-557/umars
            bank: "ibc/783F5F8F6B41874487C3B09A2306FD5E59B9B740F930A39DD55B08CF7CB8CBF0",
            // full ibc route: transfer/channel-557/umars
            dex: "ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580",
        },
    }
}
define_currency!(Mars, MARS, 6);

define_symbol! {
    TIA {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/utia
            bank: "ibc/NA_TIA",
            // full ibc route: transfer/channel-???/utia
            dex: "ibc/NA_TIA_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/utia
            bank: "ibc/NA_TIA",
            // full ibc route: transfer/channel-???/utia
            dex: "ibc/NA_TIA_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-6994/utia
            bank: "ibc/6C349F0EB135C5FA99301758F35B87DB88403D690E5E314AB080401FEE4066E5",
            // full ibc route: transfer/channel-6994/utia
            dex: "ibc/D79E7D83AB399BFFF93433E54FAA480C191248FC556924A2A8351AE2638B3877",
        },
    }
}
define_currency!(Tia, TIA, 6);

define_symbol! {
    ST_TIA {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/stutia
            bank: "ibc/NA_ST_TIA",
            // full ibc route: transfer/channel-???/stutia
            dex: "ibc/NA_ST_TIA_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/stutia
            bank: "ibc/NA_ST_TIA",
            // full ibc route: transfer/channel-???/stutia
            dex: "ibc/NA_ST_TIA_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-326/stutia
            bank: "ibc/8D4FC51F696E03711B9B37A5787FB89BD2DDBAF788813478B002D552A12F9157",
            // full ibc route: transfer/channel-326/stutia
            dex: "ibc/698350B8A61D575025F3ED13E9AC9C0F45C89DEFE92F76D5838F1D3C1A7FF7C9",
        },
    }
}
define_currency!(StTia, ST_TIA, 6);

define_symbol! {
    JKL {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/ujkl
            bank: "ibc/NA_JKL",
            // full ibc route: transfer/channel-???/ujkl
            dex: "ibc/NA_JKL_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/ujkl
            bank: "ibc/NA_JKL",
            // full ibc route: transfer/channel-???/ujkl
            dex: "ibc/NA_JKL_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-412/ujkl
            bank: "ibc/28F026607184B151F1F7D7F5D8AE644528550EB05203A28B6233DFA923669876",
            // full ibc route: transfer/channel-412/ujkl
            dex: "ibc/8E697BDABE97ACE8773C6DF7402B2D1D5104DD1EEABE12608E3469B7F64C15BA",
        },
    }
}
define_currency!(Jkl, JKL, 6);

define_symbol! {
    MILK_TIA {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/umilkTIA
            bank: "ibc/NA_MILK_TIA",
            dex: "factory/NA_MILK_TIA_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/umilkTIA
            bank: "ibc/NA_MILK_TIA",
            dex: "factory/NA_MILK_TIA_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/factory/osmo1f5vfcph2dvfeqcqkhetwv75fda69z7e5c2dldm3kvgj23crkv6wqcn47a0/umilkTIA
            bank: "ibc/16065EE5282C5217685C8F084FC44864C25C706AC37356B0D62811D50B96920F",
             dex: "factory/osmo1f5vfcph2dvfeqcqkhetwv75fda69z7e5c2dldm3kvgj23crkv6wqcn47a0/umilkTIA",
        },
    }
}
define_currency!(MilkTia, MILK_TIA, 6);

define_symbol! {
    LVN {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/ulvn
            bank: "ibc/NA_LVN",
            dex: "factory/NA_LVN_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/ulvn
            bank: "ibc/NA_LVN",
            dex: "factory/NA_LVN_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/factory/osmo1mlng7pz4pnyxtpq0akfwall37czyk9lukaucsrn30ameplhhshtqdvfm5c/ulvn
            bank: "ibc/4786BEBBFDD989C467C4552AD73065D8B2578230B8428B3B9275D540EB04C851",
            dex: "factory/osmo1mlng7pz4pnyxtpq0akfwall37czyk9lukaucsrn30ameplhhshtqdvfm5c/ulvn",
        },
    }
}
define_currency!(Lvn, LVN, 6);

define_symbol! {
    QSR {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/uqsr
            bank: "ibc/NA_QSR",
            // full ibc route: transfer/channel-???/uqsr
            dex: "ibc/NA_QSR_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/uqsr
            bank: "ibc/NA_QSR",
            // full ibc route: transfer/channel-???/uqsr
            dex: "ibc/NA_QSR_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-688/uqsr
            bank: "ibc/FF456FD21AA44251D2122BF19B20C5FE717A1EBD054A59FA1CA4B21742048CA0",
            // full ibc route: transfer/channel-688/uqsr
            dex: "ibc/1B708808D372E959CD4839C594960309283424C775F4A038AAEBE7F83A988477",
        },
    }
}
define_currency!(Qsr, QSR, 6);

define_symbol! {
    PICA {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/ppica
            bank: "ibc/NA_PICA",
            // full ibc route: transfer/channel-???/ppica
            dex: "ibc/NA_PICA_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/ppica
            bank: "ibc/NA_PICA",
            // full ibc route: transfer/channel-???/ppica
            dex: "ibc/NA_PICA_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-1279/ppica
            bank: "ibc/7F2DC2A595EDCAEC1C03D607C6DC3C79EDDC029A53D16C0788835C1A9AA06306",
            // full ibc route: transfer/channel-1279/ppica
            dex: "ibc/56D7C03B8F6A07AD322EEE1BEF3AE996E09D1C1E34C27CF37E0D4A0AC5972516",
        },
    }
}
define_currency!(Pica, PICA, 12);

define_symbol! {
    DYM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/adym
            bank: "ibc/NA_DYM",
            // full ibc route: transfer/channel-???/adym
            dex: "ibc/NA_DYM_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/adym
            bank: "ibc/NA_DYM",
            // full ibc route: transfer/channel-???/adym
            dex: "ibc/NA_DYM_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-19774/adym
            bank: "ibc/9C7F70E92CCBA0F2DC94796B0682955E090676EA7A2F8E0A4611956B79CB4406",
            // full ibc route: transfer/channel-19774/adym
            dex: "ibc/9A76CDF0CBCEF37923F32518FA15E5DC92B9F56128292BC4D63C4AEA76CBB110",
        },
    }
}
define_currency!(Dym, DYM, 18);

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
    maybe_visit::<_, Atom, _>(matcher, symbol, visitor)
        .or_else(|visitor| maybe_visit::<_, StAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Osmo, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, StOsmo, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Axl, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, QAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, StkAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Strd, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Inj, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Secret, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Stars, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Cro, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Juno, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Evmos, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Mars, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Tia, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, StTia, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Jkl, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, MilkTia, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Lvn, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Qsr, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Pica, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Dym, _>(matcher, symbol, visitor))
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {lease::LeaseGroup, lpn::Lpn, native::osmosis::Nls},
    };

    use super::{Atom, Dym, Lvn, Osmo, Pica, Qsr, StAtom, StOsmo, StTia, Tia, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Tia, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StTia, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Lvn, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Dym, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Lpn::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Tia, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Pica, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Qsr, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Dym, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, LeaseGroup>(Lpn::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::TICKER);
    }
}
