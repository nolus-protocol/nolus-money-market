use sdk::schemars;

use crate::{define_currency, define_symbol};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-12/uatom
            bank: "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196",
            /// full ibc route: transfer/channel-12/uatom
            dex: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-0/uatom
            bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
            /// full ibc route: transfer/channel-0/uatom
            dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        },
    }
}
define_currency!(Atom, ATOM);

define_symbol! {
    ST_ATOM {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/uatom
            bank: "ibc/NA_ST_ATOM",
            /// full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_ATOM_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-326/stuatom
            bank: "ibc/FCFF8B19C61677F3B78E2A5AE3B4A34A8D23858D16905F253B8438B3AFD07FF8",
            /// full ibc route: transfer/channel-326/stuatom
            dex: "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901",
        },
    }
}
define_currency!(StAtom, ST_ATOM);

define_symbol! {
    OSMO {
        ["dev", "test", "main"]: {
            /// full ibc route: transfer/channel-0/uosmo
            bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
            dex: "uosmo",
        },
    }
}
define_currency!(Osmo, OSMO);

define_symbol! {
    ST_OSMO {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/uatom
            bank: "ibc/NA_ST_OSMO",
            /// full ibc route: transfer/channel-??/uatom
            dex: "ibc/NA_ST_OSMO_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-326/stuosmo
            bank: "ibc/AF5559D128329B6C753F15481BEC26E533B847A471074703FA4903E7E6F61BA1",
            /// full ibc route: transfer/channel-326/stuosmo
            dex: "ibc/D176154B0C63D1F9C6DCFB4F70349EBF2E2B5A87A05902F57A6AE92B863E9AEC",
        },
    }
}
define_currency!(StOsmo, ST_OSMO);

define_symbol! {
    WETH {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-3/eth-wei
            bank: "ibc/98CD37B180F06F954AFC71804049BE6EEA2A3B0CCEA1F425D141245BCFFBBD33",
            /// full ibc route: transfer/channel-3/eth-wei
            /// channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            /// although there is no pool WETH participates in
            dex: "ibc/29320BE25C3BF64A2355344625410899C1EB164038E328531C36095B0AA8BBFC",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
            bank: "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C",
            /// full ibc route: transfer/channel-208/weth-wei
            dex: "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5",
        },
    }
}
define_currency!(Weth, WETH);

define_symbol! {
    WBTC {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-3/btc-satoshi
            bank: "ibc/680E95D3CEA378B7302926B8A5892442F1F7DF78E22199AE248DCBADC9A0C1A2",
            /// full ibc route: transfer/channel-3/btc-satoshi
            /// channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            /// although there is no denomination trace as per `osmosisd q ibc-transfer denom-trace`
            dex: "ibc/CEDA3AFF171E72ACB689B7B64E988C0077DA7D4BF157637FFBDEB688D205A473",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
            bank: "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
            /// full ibc route: transfer/channel-208/wbtc-satoshi
            dex: "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
        },
    }
}
define_currency!(Wbtc, WBTC);

define_symbol! {
    AKT {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-73/uakt
            bank: "ibc/1064EED4A8E99F9C1158680236D0C5C3EA6B8BB65C9F87DAC6BC759DD904D818",
            /// full ibc route: transfer/channel-73/uakt
            dex: "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-1/uakt
            bank: "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459",
            /// full ibc route: transfer/channel-1/uakt
            dex: "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4",
        },
    }
}
define_currency!(Akt, AKT);

define_symbol! {
    AXL {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-3/uaxl
            /// not in use due to the lack of a pool
            bank: "ibc/NA_AXL",
            /// full ibc route: transfer/channel-3/uaxl
            /// not in use due to the lack of a pool
            dex: "ibc/NA_AXL_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-208/uaxl
            bank: "ibc/1B03A71B8E6F6EF424411DC9326A8E0D25D096E4D2616425CFAF2AF06F0FE717",
            /// full ibc route: transfer/channel-208/uaxl
            dex: "ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E",
        },
    }
}
define_currency!(Axl, AXL);

define_symbol! {
    Q_ATOM {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/uqatom
            bank: "ibc/NA_Q_ATOM",
            /// full ibc route: transfer/channel-??/uqatom
            dex: "ibc/NA_Q_ATOM_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-522/uqatom
            bank: "ibc/317FCA2D7554F55BBCD0019AB36F7FEA18B6D161F462AF5E565068C719A29F20",
            /// full ibc route: transfer/channel-522/uqatom
            dex: "ibc/FA602364BEC305A696CBDF987058E99D8B479F0318E47314C49173E8838C5BAC",
        },
    }
}
define_currency!(QAtom, Q_ATOM);

define_symbol! {
    STK_ATOM {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/stk/uatom
            bank: "ibc/NA_STK_ATOM",
            /// full ibc route: transfer/channel-??/stk/uatom
            dex: "ibc/NA_STK_ATOM_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-4/stk/uatom
            bank: "ibc/DAAD372DB7DD45BBCFA4DDD40CA9793E9D265D1530083AB41A8A0C53C3EBE865",
            /// full ibc route: transfer/channel-4/stk/uatom
            dex: "ibc/CAA179E40F0266B0B29FB5EAA288FB9212E628822265D4141EBD1C47C3CBFCBC",
        },
    }
}
define_currency!(StkAtom, STK_ATOM);

define_symbol! {
    STRD {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/ustrd
            bank: "ibc/NA_STRD",
            /// full ibc route: transfer/channel-??/ustrd
            dex: "ibc/NA_STRD_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-326/ustrd
            bank: "ibc/04CA9067228BB51F1C39A506DA00DF07E1496D8308DD21E8EF66AD6169FA722B",
            /// full ibc route: transfer/channel-326/ustrd
            dex: "ibc/A8CA5EE328FA10C9519DF6057DA1F69682D28F7D0F5CCC7ECB72E3DCA2D157A4",
        },
    }
}
define_currency!(Strd, STRD);

define_symbol! {
    INJ {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/inj
            bank: "ibc/NA_INJ",
            /// full ibc route: transfer/channel-??/inj
            dex: "ibc/NA_INJ_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-122/inj
            bank: "ibc/4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E",
            /// full ibc route: transfer/channel-122/inj
            dex: "ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273",
        },
    }
}
define_currency!(Inj, INJ);

define_symbol! {
    SCRT {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/uscrt
            bank: "ibc/NA_SCRT",
            /// full ibc route: transfer/channel-??/uscrt
            dex: "ibc/NA_SCRT_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-88/uscrt
            bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
            /// full ibc route: transfer/channel-88/uscrt
            dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
        },
    }
}
define_currency!(Secret, SCRT);

define_symbol! {
    STARS {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/ustars
            bank: "ibc/NA_STARS",
            /// full ibc route: transfer/channel-??/ustars
            dex: "ibc/NA_STARS_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-75/ustars
            bank: "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5",
            /// full ibc route: transfer/channel-75/ustars
            dex: "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
        },
    }
}
define_currency!(Stars, STARS);

define_symbol! {
    CRO {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/basecro
            bank: "ibc/NA_CRO",
            /// full ibc route: transfer/channel-??/basecro
            dex: "ibc/NA_CRO_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-5/basecro
            bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
            // full ibc route: transfer/channel-5/basecro
            dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
        },
    }
}
define_currency!(Cro, CRO);

define_symbol! {
    JUNO {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-1/ujunox
            bank: "ibc/8FB044422997A8A77891DE729EC28638DDE4C81A54398F68149A058AA9B74D9F",
            /// full ibc route: transfer/channel-1/ujunox
            dex: "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-42/ujuno
            bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
            /// full ibc route: transfer/channel-42/ujuno
            dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        },
    }
}
define_currency!(Juno, JUNO);

define_symbol! {
    EVMOS {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-??/aevmos
            bank: "ibc/NA_EVMOS",
            /// full ibc route: transfer/channel-??/aevmos
            dex: "ibc/NA_EVMOS_DEX",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-204/aevmos
            bank: "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE",
            /// full ibc route: transfer/channel-204/aevmos
            dex: "ibc/6AE98883D4D5D5FF9E50D7130F1305DA2FFA0C652D1DD9C123657C6B4EB2DF8A",
        },
    }
}
define_currency!(Evmos, EVMOS);

define_symbol! {
    MARS {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-24/umars
            bank: "ibc/1CC042AD599E184C0F77DC5D89443C82F8A16B6E13DEC650A7A50A5D0AA330C3",
            /// full ibc route: transfer/channel-24/umars
            dex: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-557/umars
            bank: "ibc/783F5F8F6B41874487C3B09A2306FD5E59B9B740F930A39DD55B08CF7CB8CBF0",
            /// full ibc route: transfer/channel-557/umars
            dex: "ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580",
        },
    }
}
define_currency!(Mars, MARS);

#[cfg(test)]
mod test {

    use crate::{
        dex::test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        lease::LeaseGroup,
        lpn::osmosis::Usdc,
        native::osmosis::Nls,
        Currency,
    };

    use super::{Atom, Osmo, StAtom, StOsmo, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_err::<Usdc, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Usdc::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Usdc, LeaseGroup>(Usdc::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::TICKER);
    }
}
