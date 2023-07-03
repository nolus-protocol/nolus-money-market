use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    currency::{AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic},
    define_currency, define_symbol, SingleVisitorAdapter,
};

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
            dex: "ibc/NA_ST_ATOM",
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
            dex: "ibc/NA_ST_OSMO",
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
    EVMOS {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-227/atevmos
            bank: "ibc/E716E3AA644A2225D13AC25196F13A4B9BBB518EF059F31ED6CAF26D157C4870",
            /// full ibc route: transfer/channel-227/atevmos
            dex: "ibc/3A7AC1F623B3475EE1F3CF849FBC4751FCEB956327ED4E5D49C676828EF9533E",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-204/aevmos
            bank: "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE",
            /// full ibc route: transfer/channel-204/aevmos
            dex: "ibc/6AE98883D4D5D5FF9E50D7130F1305DA2FFA0C652D1DD9C123657C6B4EB2DF8A",
        },
    }
}
#[cfg(feature = "testing")]
define_currency!(Evmos, EVMOS);

define_symbol! {
    JUNO {
        ["dev", "test", "main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-42/ujuno
            bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
            /// full ibc route: transfer/channel-42/ujuno
            dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        },
    }
}
#[cfg(feature = "testing")]
define_currency!(Juno, JUNO);

define_symbol! {
    STARS {
        ["dev", "test", "main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-75/ustars
            bank: "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5",
            /// full ibc route: transfer/channel-75/ustars
            dex: "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
        },
    }
}
#[cfg(feature = "testing")]
define_currency!(Stars, STARS);

define_symbol! {
    CRO {
        ["dev", "test", "main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-5/basecro
            bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
            // full ibc route: transfer/channel-5/basecro
            dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
        },
    }
}
#[cfg(feature = "testing")]
define_currency!(Cro, CRO);

define_symbol! {
    SCRT {
        ["dev", "test", "main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-88/uscrt
            bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
            /// full ibc route: transfer/channel-88/uscrt
            dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
        },
    }
}
#[cfg(feature = "testing")]
define_currency!(Secret, SCRT);

#[derive(Clone, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: SymbolStatic = "lease";

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        use crate::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        let r = maybe_visit::<Atom, _>(ticker, v)
            .or_else(|v| maybe_visit::<StAtom, _>(ticker, v))
            .or_else(|v| maybe_visit::<Osmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<StOsmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<Weth, _>(ticker, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(ticker, v));

        #[cfg(feature = "testing")]
        let r = r
            .or_else(|v| maybe_visit::<Evmos, _>(ticker, v))
            .or_else(|v| maybe_visit::<Juno, _>(ticker, v))
            .or_else(|v| maybe_visit::<Stars, _>(ticker, v))
            .or_else(|v| maybe_visit::<Cro, _>(ticker, v))
            .or_else(|v| maybe_visit::<Secret, _>(ticker, v));

        r.map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        use crate::currency::maybe_visit_on_bank_symbol as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        let r = maybe_visit::<Atom, _>(bank_symbol, v)
            .or_else(|v| maybe_visit::<StAtom, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Osmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<StOsmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Weth, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(bank_symbol, v));

        #[cfg(feature = "testing")]
        let r = r
            .or_else(|v| maybe_visit::<Evmos, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Juno, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Stars, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Cro, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Secret, _>(bank_symbol, v));

        r.map_err(|v| v.0)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        lease::Osmo,
        lpn::Usdc,
        native::Nls,
        test::group::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        Currency,
    };

    use super::{Atom, LeaseGroup, StAtom, StOsmo, Wbtc, Weth};

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
