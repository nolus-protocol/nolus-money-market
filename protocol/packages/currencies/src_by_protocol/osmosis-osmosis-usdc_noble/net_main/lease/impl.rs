use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult};
use sdk::schemars;

use crate::{define_currency, define_symbol, LeaseGroup};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-0/uatom
        bank: "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388",
        // full ibc route: transfer/channel-0/uatom
        dex: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
    }
}
define_currency!(Atom, ATOM, LeaseGroup, 6);

define_symbol! {
    ST_ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-326/stuatom
        bank: "ibc/FCFF8B19C61677F3B78E2A5AE3B4A34A8D23858D16905F253B8438B3AFD07FF8",
        // full ibc route: transfer/channel-326/stuatom
        dex: "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901",
    }
}
define_currency!(StAtom, ST_ATOM, LeaseGroup, 6);

define_symbol! {
    OSMO {
        // full ibc route: transfer/channel-0/uosmo
        bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
        dex: "uosmo",
    }
}
define_currency!(Osmo, OSMO, LeaseGroup, 6);

define_symbol! {
    ST_OSMO {
        // full ibc route: transfer/channel-0/transfer/channel-326/stuosmo
        bank: "ibc/AF5559D128329B6C753F15481BEC26E533B847A471074703FA4903E7E6F61BA1",
        // full ibc route: transfer/channel-326/stuosmo
        dex: "ibc/D176154B0C63D1F9C6DCFB4F70349EBF2E2B5A87A05902F57A6AE92B863E9AEC",
    }
}
define_currency!(StOsmo, ST_OSMO, LeaseGroup, 6);

define_symbol! {
    WETH {
        // full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
        bank: "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C",
        // full ibc route: transfer/channel-208/weth-wei
        dex: "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5",
    }
}
define_currency!(Weth, WETH, LeaseGroup, 18);

define_symbol! {
    WBTC {
        // full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
        bank: "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221",
        // full ibc route: transfer/channel-208/wbtc-satoshi
        dex: "ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F",
    }
}
define_currency!(Wbtc, WBTC, LeaseGroup, 8);

define_symbol! {
    AKT {
        // full ibc route: transfer/channel-0/transfer/channel-1/uakt
        bank: "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459",
        // full ibc route: transfer/channel-1/uakt
        dex: "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4",
    }
}
define_currency!(Akt, AKT, LeaseGroup, 6);

define_symbol! {
    AXL {
        // full ibc route: transfer/channel-0/transfer/channel-208/uaxl
        bank: "ibc/1B03A71B8E6F6EF424411DC9326A8E0D25D096E4D2616425CFAF2AF06F0FE717",
        // full ibc route: transfer/channel-208/uaxl
        dex: "ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E",
    }
}
define_currency!(Axl, AXL, LeaseGroup, 6);

define_symbol! {
    Q_ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-522/uqatom
        bank: "ibc/317FCA2D7554F55BBCD0019AB36F7FEA18B6D161F462AF5E565068C719A29F20",
        // full ibc route: transfer/channel-522/uqatom
        dex: "ibc/FA602364BEC305A696CBDF987058E99D8B479F0318E47314C49173E8838C5BAC",
    }
}
define_currency!(QAtom, Q_ATOM, LeaseGroup, 6);

define_symbol! {
    STK_ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-4/stk/uatom
        bank: "ibc/DAAD372DB7DD45BBCFA4DDD40CA9793E9D265D1530083AB41A8A0C53C3EBE865",
        // full ibc route: transfer/channel-4/stk/uatom
        dex: "ibc/CAA179E40F0266B0B29FB5EAA288FB9212E628822265D4141EBD1C47C3CBFCBC",
    }
}
define_currency!(StkAtom, STK_ATOM, LeaseGroup, 6);

define_symbol! {
    STRD {
        // full ibc route: transfer/channel-0/transfer/channel-326/ustrd
        bank: "ibc/04CA9067228BB51F1C39A506DA00DF07E1496D8308DD21E8EF66AD6169FA722B",
        // full ibc route: transfer/channel-326/ustrd
        dex: "ibc/A8CA5EE328FA10C9519DF6057DA1F69682D28F7D0F5CCC7ECB72E3DCA2D157A4",
    }
}
define_currency!(Strd, STRD, LeaseGroup, 6);

define_symbol! {
    INJ {
        // full ibc route: transfer/channel-0/transfer/channel-122/inj
        bank: "ibc/4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E",
        // full ibc route: transfer/channel-122/inj
        dex: "ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273",
    }
}
define_currency!(Inj, INJ, LeaseGroup, 18);

define_symbol! {
    SCRT {
        // full ibc route: transfer/channel-0/transfer/channel-88/uscrt
        bank: "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107",
        // full ibc route: transfer/channel-88/uscrt
        dex: "ibc/0954E1C28EB7AF5B72D24F3BC2B47BBB2FDF91BDDFD57B74B99E133AED40972A",
    }
}
define_currency!(Secret, SCRT, LeaseGroup, 6);

define_symbol! {
    STARS {
        // full ibc route: transfer/channel-0/transfer/channel-75/ustars
        bank: "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5",
        // full ibc route: transfer/channel-75/ustars
        dex: "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
    }
}
define_currency!(Stars, STARS, LeaseGroup, 6);

define_symbol! {
    CRO {
        // full ibc route: transfer/channel-0/transfer/channel-5/basecro
        bank: "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48",
        // full ibc route: transfer/channel-5/basecro
        dex: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1",
    }
}
define_currency!(Cro, CRO, LeaseGroup, 8);

define_symbol! {
    JUNO {
        // full ibc route: transfer/channel-0/transfer/channel-42/ujuno
        bank: "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C",
        // full ibc route: transfer/channel-42/ujuno
        dex: "ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
    }
}
define_currency!(Juno, JUNO, LeaseGroup, 6);

define_symbol! {
    EVMOS {
        // full ibc route: transfer/channel-0/transfer/channel-204/aevmos
        bank: "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE",
        // full ibc route: transfer/channel-204/aevmos
        dex: "ibc/6AE98883D4D5D5FF9E50D7130F1305DA2FFA0C652D1DD9C123657C6B4EB2DF8A",
    }
}
define_currency!(Evmos, EVMOS, LeaseGroup, 18);

define_symbol! {
    MARS {
        // full ibc route: transfer/channel-0/transfer/channel-557/umars
        bank: "ibc/783F5F8F6B41874487C3B09A2306FD5E59B9B740F930A39DD55B08CF7CB8CBF0",
        // full ibc route: transfer/channel-557/umars
        dex: "ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580",
    }
}
define_currency!(Mars, MARS, LeaseGroup, 6);

define_symbol! {
    TIA {
        // full ibc route: transfer/channel-0/transfer/channel-6994/utia
        bank: "ibc/6C349F0EB135C5FA99301758F35B87DB88403D690E5E314AB080401FEE4066E5",
        // full ibc route: transfer/channel-6994/utia
        dex: "ibc/D79E7D83AB399BFFF93433E54FAA480C191248FC556924A2A8351AE2638B3877",
    }
}
define_currency!(Tia, TIA, LeaseGroup, 6);

define_symbol! {
    ST_TIA {
        // full ibc route: transfer/channel-0/transfer/channel-326/stutia
        bank: "ibc/8D4FC51F696E03711B9B37A5787FB89BD2DDBAF788813478B002D552A12F9157",
        // full ibc route: transfer/channel-326/stutia
        dex: "ibc/698350B8A61D575025F3ED13E9AC9C0F45C89DEFE92F76D5838F1D3C1A7FF7C9",
    }
}
define_currency!(StTia, ST_TIA, LeaseGroup, 6);

define_symbol! {
    JKL {
        // full ibc route: transfer/channel-0/transfer/channel-412/ujkl
        bank: "ibc/28F026607184B151F1F7D7F5D8AE644528550EB05203A28B6233DFA923669876",
        // full ibc route: transfer/channel-412/ujkl
        dex: "ibc/8E697BDABE97ACE8773C6DF7402B2D1D5104DD1EEABE12608E3469B7F64C15BA",
    }
}
define_currency!(Jkl, JKL, LeaseGroup, 6);

define_symbol! {
    MILK_TIA {
        // full ibc route: transfer/channel-0/factory/osmo1f5vfcph2dvfeqcqkhetwv75fda69z7e5c2dldm3kvgj23crkv6wqcn47a0/umilkTIA
        bank: "ibc/16065EE5282C5217685C8F084FC44864C25C706AC37356B0D62811D50B96920F",
        dex: "factory/osmo1f5vfcph2dvfeqcqkhetwv75fda69z7e5c2dldm3kvgj23crkv6wqcn47a0/umilkTIA",
    }
}
define_currency!(MilkTia, MILK_TIA, LeaseGroup, 6);

define_symbol! {
    LVN {
        // full ibc route: transfer/channel-0/factory/osmo1mlng7pz4pnyxtpq0akfwall37czyk9lukaucsrn30ameplhhshtqdvfm5c/ulvn
        bank: "ibc/4786BEBBFDD989C467C4552AD73065D8B2578230B8428B3B9275D540EB04C851",
        dex: "factory/osmo1mlng7pz4pnyxtpq0akfwall37czyk9lukaucsrn30ameplhhshtqdvfm5c/ulvn",
    }
}
define_currency!(Lvn, LVN, LeaseGroup, 6);

define_symbol! {
    QSR {
        // full ibc route: transfer/channel-0/transfer/channel-688/uqsr
        bank: "ibc/FF456FD21AA44251D2122BF19B20C5FE717A1EBD054A59FA1CA4B21742048CA0",
        // full ibc route: transfer/channel-688/uqsr
        dex: "ibc/1B708808D372E959CD4839C594960309283424C775F4A038AAEBE7F83A988477",
    }
}
define_currency!(Qsr, QSR, LeaseGroup, 6);

define_symbol! {
    PICA {
        // full ibc route: transfer/channel-0/transfer/channel-1279/ppica
        bank: "ibc/7F2DC2A595EDCAEC1C03D607C6DC3C79EDDC029A53D16C0788835C1A9AA06306",
        // full ibc route: transfer/channel-1279/ppica
        dex: "ibc/56D7C03B8F6A07AD322EEE1BEF3AE996E09D1C1E34C27CF37E0D4A0AC5972516",
    }
}
define_currency!(Pica, PICA, LeaseGroup, 12);

define_symbol! {
    DYM {
        // full ibc route: transfer/channel-0/transfer/channel-19774/adym
        bank: "ibc/9C7F70E92CCBA0F2DC94796B0682955E090676EA7A2F8E0A4611956B79CB4406",
        // full ibc route: transfer/channel-19774/adym
        dex: "ibc/9A76CDF0CBCEF37923F32518FA15E5DC92B9F56128292BC4D63C4AEA76CBB110",
    }
}
define_currency!(Dym, DYM, LeaseGroup, 18);

define_symbol! {
    CUDOS {
        // full ibc route: transfer/channel-0/transfer/channel-298/acudos
        bank: "ibc/BB9810E7FE8836311126F15BE0B20E7463189751840F8C3FEF3AC8F87D8AB7C8",
        // full ibc route: transfer/channel-298/acudos
        dex: "ibc/E09ED39F390EC51FA9F3F69BEA08B5BBE6A48B3057B2B1C3467FAAE9E58B021B",
    }
}
define_currency!(Cudos, CUDOS, LeaseGroup, 18);

define_symbol! {
    SAGA {
        // full ibc route: transfer/channel-0/transfer/channel-38946/usaga
        bank: "ibc/4C3767D90875C31870ACAC95C349BBFD0585FFF8486C14F6BA0B0ED8D7D35CFD",
        // full ibc route: transfer/channel-38946/usaga
        dex: "ibc/094FB70C3006906F67F5D674073D2DAFAFB41537E7033098F5C752F211E7B6C2",
    }
}
define_currency!(Saga, SAGA, LeaseGroup, 6);

pub(super) fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, Atom, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, StAtom, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Osmo, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, StOsmo, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Axl, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, QAtom, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Strd, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Inj, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Secret, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Stars, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Cro, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Juno, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Evmos, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Mars, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Tia, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, StTia, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Jkl, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, MilkTia, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Lvn, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Qsr, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Pica, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Dym, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Cudos, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Saga, _>(matcher, visitor))
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {lease::LeaseGroup, lpn::Lpn, native::Nls},
    };

    use super::{
        Atom, Cudos, Dym, Lvn, Osmo, Pica, Qsr, Saga, StAtom, StOsmo, StTia, Tia, Wbtc, Weth,
    };

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
        maybe_visit_on_ticker_impl::<Cudos, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Saga, LeaseGroup>();
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
