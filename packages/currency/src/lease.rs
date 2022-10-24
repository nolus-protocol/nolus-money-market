use serde::{Deserialize, Serialize};

use finance::currency::{AnyVisitor, Currency, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};

use crate::{lpn::Usdc, SingleVisitorAdapter};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Atom {}
impl Currency for Atom {
    const TICKER: SymbolStatic = "ATOM";
    /// full ibc route: transfer/channel-0/transfer/channel-0/uatom
    const BANK_SYMBOL: SymbolStatic =
        "ibc/6CDD4663F2F09CD62285E2D45891FC149A3568E316CE3EBBE201A71A78A69388";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Osmo {}
impl Currency for Osmo {
    const TICKER: SymbolStatic = "OSMO";
    /// full ibc route: transfer/channel-0/uosmo
    const BANK_SYMBOL: SymbolStatic =
        "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Weth {}
impl Currency for Weth {
    const TICKER: SymbolStatic = "WETH";
    /// full ibc route: transfer/channel-0/transfer/channel-208/weth-wei
    const BANK_SYMBOL: SymbolStatic =
        "ibc/A7C4A3FB19E88ABE60416125F9189DA680800F4CDD14E3C10C874E022BEFF04C";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Wbtc {}
impl Currency for Wbtc {
    const TICKER: SymbolStatic = "WBTC";
    /// full ibc route: transfer/channel-0/transfer/channel-208/wbtc-satoshi
    const BANK_SYMBOL: SymbolStatic =
        "ibc/84E70F4A34FB2DE135FD3A04FDDF53B7DA4206080AA785C8BAB7F8B26299A221";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Evmos {}
impl Currency for Evmos {
    const TICKER: SymbolStatic = "EVMOS";
    /// full ibc route: transfer/channel-0/transfer/channel-204/aevmos
    const BANK_SYMBOL: SymbolStatic =
        "ibc/A59A9C955F1AB8B76671B00C1A0482C64A6590352944BB5880E5122358F7E1CE";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Juno {}
impl Currency for Juno {
    const TICKER: SymbolStatic = "JUNO";
    /// full ibc route: transfer/channel-0/transfer/channel-42/ujuno
    const BANK_SYMBOL: SymbolStatic =
        "ibc/4F3E83AB35529435E4BFEA001F5D935E7250133347C4E1010A9C77149EF0394C";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Stars {}
impl Currency for Stars {
    const TICKER: SymbolStatic = "STARS";
    /// full ibc route: transfer/channel-0/transfer/channel-75/ustars
    const BANK_SYMBOL: SymbolStatic =
        "ibc/11E3CF372E065ACB1A39C531A3C7E7E03F60B5D0653AD2139D31128ACD2772B5";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Cro {}
impl Currency for Cro {
    const TICKER: SymbolStatic = "CRO";
    /// full ibc route: transfer/channel-0/transfer/channel-5/basecro
    const BANK_SYMBOL: SymbolStatic =
        "ibc/E1BCC0F7B932E654B1A930F72B76C0678D55095387E2A4D8F00E941A8F82EE48";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Secret {}
impl Currency for Secret {
    const TICKER: SymbolStatic = "SCRT";
    /// full ibc route: transfer/channel-0/transfer/channel-88/uscrt
    const BANK_SYMBOL: SymbolStatic =
        "ibc/EA00FFF0335B07B5CD1530B7EB3D2C710620AE5B168C71AFF7B50532D690E107";
}

pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: SymbolStatic = "lease";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
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
            // TODO REMOVE once migrate off the single currency version
            .or_else(|v| maybe_visit::<Usdc, _>(ticker, v))
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
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
            // TODO REMOVE once migrate off the single currency version
            .or_else(|v| maybe_visit::<Usdc, _>(bank_symbol, v))
            .map_err(|v| v.0)
    }
}

#[cfg(test)]
mod test {
    use finance::currency::Currency;

    use crate::{
        lease::Osmo,
        native::Nls,
        test::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{Atom, Cro, Evmos, Juno, LeaseGroup, Secret, Stars, Usdc, Wbtc, Weth};

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
        maybe_visit_on_ticker_impl::<Usdc, LeaseGroup>(); // TODO REMOVE once migrate off the single currency version
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
        maybe_visit_on_bank_symbol_impl::<Usdc, LeaseGroup>(); // TODO REMOVE once migrate off the single currency version
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
    }
}
