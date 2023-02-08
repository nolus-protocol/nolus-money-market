use finance::currency::{AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

use crate::{define_currency, define_symbol, SingleVisitorAdapter};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-0/uatom
            bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
            /// full ibc route: transfer/channel-0/uatom
            dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-0/uatom
            bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
            /// full ibc route: transfer/channel-0/uatom
            dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
        },
    }
}
define_currency!(Atom, ATOM);

define_symbol! {
    OSMO {
        {
            /// full ibc route: transfer/channel-0/uosmo
            bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
            dex: "uosmo",
        },
        alt: {
            /// full ibc route: transfer/channel-0/uosmo
            bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
            dex: "uosmo",
        },
    }
}
define_currency!(Osmo, OSMO);

define_symbol! {
    WETH {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
            bank: "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C",
            /// full ibc route: transfer/channel-208/weth-wei
            dex: "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-312/eth-wei
            bank: "ibc/E402E4FDD236172DB494E3E8A38D97BE641DD2CE2D089C1F116F5897CDBD40E9",
            /// full ibc route: transfer/channel-312/eth-wei
            /// channel-312 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            /// with WETH currency listed as `eth-wei`
            dex: "ibc/8AE11672A7DF38BF7B484AB642C5C85BA4A94810D57AE8945151818CD6179427",
        },
    }
}
define_currency!(Weth, WETH);

define_symbol! {
    WBTC {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
            bank: "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
            /// full ibc route: transfer/channel-208/wbtc-satoshi
            dex: "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-312/btc-satoshi
            bank: "ibc/215A3334E07EAE7E7B47617272472CAF903E99EDE3A80A2CF7CA8BE1F761AC68",
            /// full ibc route: transfer/channel-312/btc-satoshi
            /// channel-312 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
            /// but there is no WBTC currency listed
            dex: "ibc/BDA12A41BCF2DFB005A0794876E0E71D9538E0A7EB9607F600435EDDE5393EC4",
        },
    }
}
define_currency!(Wbtc, WBTC);

define_symbol! {
    EVMOS {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-204/aevmos
            bank: "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE",
            /// full ibc route: transfer/channel-204/aevmos
            dex: "ibc/6AE98883D4D5D5FF9E50D7130F1305DA2FFA0C652D1DD9C123657C6B4EB2DF8A",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-227/atevmos
            bank: "ibc/E716E3AA644A2225D13AC25196F13A4B9BBB518EF059F31ED6CAF26D157C4870",
            /// full ibc route: transfer/channel-227/atevmos
            dex: "ibc/3A7AC1F623B3475EE1F3CF849FBC4751FCEB956327ED4E5D49C676828EF9533E",
        },
    }
}
define_currency!(Evmos, EVMOS);

define_symbol! {
    JUNO {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-42/ujuno
            bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
            /// full ibc route: transfer/channel-42/ujuno
            dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-42/ujuno
            bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
            /// full ibc route: transfer/channel-42/ujuno
            dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        },
    }
}
define_currency!(Juno, JUNO);

define_symbol! {
    STARS {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-75/ustars
            bank: "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5",
            /// full ibc route: transfer/channel-75/ustars
            dex: "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
        },
        alt: {
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
        {
            /// full ibc route: transfer/channel-0/transfer/channel-5/basecro
            bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
            // full ibc route: transfer/channel-5/basecro
            dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-5/basecro
            bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
            // full ibc route: transfer/channel-5/basecro
            dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
        },
    }
}
define_currency!(Cro, CRO);

define_symbol! {
    SCRT {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-88/uscrt
            bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
            /// full ibc route: transfer/channel-88/uscrt
            dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-88/uscrt
            bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
            /// full ibc route: transfer/channel-88/uscrt
            dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
        },
    }
}
define_currency!(Secret, SCRT);

#[derive(Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: SymbolStatic = "lease";

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        use finance::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        maybe_visit::<Atom, _>(ticker, v)
            .or_else(|v| maybe_visit::<Osmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<Weth, _>(ticker, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(ticker, v))
            .or_else(|v| maybe_visit::<Evmos, _>(ticker, v))
            .or_else(|v| maybe_visit::<Juno, _>(ticker, v))
            .or_else(|v| maybe_visit::<Stars, _>(ticker, v))
            .or_else(|v| maybe_visit::<Cro, _>(ticker, v))
            .or_else(|v| maybe_visit::<Secret, _>(ticker, v))
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        use finance::currency::maybe_visit_on_bank_symbol as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        maybe_visit::<Atom, _>(bank_symbol, v)
            .or_else(|v| maybe_visit::<Osmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Weth, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Evmos, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Juno, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Stars, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Cro, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Secret, _>(bank_symbol, v))
            .map_err(|v| v.0)
    }
}

#[cfg(test)]
mod test {
    use finance::currency::Currency;

    use crate::{
        lease::Osmo,
        lpn::Usdc,
        native::Nls,
        test::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{Atom, Cro, Evmos, Juno, LeaseGroup, Secret, Stars, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Evmos, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Juno, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Stars, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Cro, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Secret, LeaseGroup>();
        maybe_visit_on_ticker_err::<Usdc, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Usdc::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Evmos, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Juno, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Stars, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Cro, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Secret, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Usdc, LeaseGroup>(Usdc::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
    }
}
