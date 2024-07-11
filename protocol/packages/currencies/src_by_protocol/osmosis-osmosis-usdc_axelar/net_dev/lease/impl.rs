use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult};
use sdk::schemars;

use crate::{define_currency, define_symbol};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-12/uatom
        bank: "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196",
        // full ibc route: transfer/channel-12/uatom
        dex: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477",
    }
}
define_currency!(Atom, ATOM, 6);

define_symbol! {
    OSMO {
        // full ibc route: transfer/channel-0/uosmo
        bank: "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
        dex: "uosmo",
    }
}
define_currency!(Osmo, OSMO, 6);

define_symbol! {
    WETH {
        // full ibc route: transfer/channel-0/transfer/channel-3/eth-wei
        bank: "ibc/98CD37B180F06F954AFC71804049BE6EEA2A3B0CCEA1F425D141245BCFFBBD33",
        // full ibc route: transfer/channel-3/eth-wei
        // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
        // although there is no pool WETH participates in
        dex: "ibc/29320BE25C3BF64A2355344625410899C1EB164038E328531C36095B0AA8BBFC",
    }
}
define_currency!(Weth, WETH, 18);

define_symbol! {
    WBTC {
        // full ibc route: transfer/channel-0/transfer/channel-3/btc-satoshi
        bank: "ibc/680E95D3CEA378B7302926B8A5892442F1F7DF78E22199AE248DCBADC9A0C1A2",
        // full ibc route: transfer/channel-3/btc-satoshi
        // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
        // although there is no denomination trace as per `osmosisd q ibc-transfer denom-trace`
        dex: "ibc/CEDA3AFF171E72ACB689B7B64E988C0077DA7D4BF157637FFBDEB688D205A473",
    }
}
define_currency!(Wbtc, WBTC, 8);

define_symbol! {
    AKT {
        // full ibc route: transfer/channel-0/transfer/channel-73/uakt
        bank: "ibc/1064EED4A8E99F9C1158680236D0C5C3EA6B8BB65C9F87DAC6BC759DD904D818",
        // full ibc route: transfer/channel-73/uakt
        dex: "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857",
    }
}
define_currency!(Akt, AKT, 6);

define_symbol! {
    JUNO {
        // full ibc route: transfer/channel-0/transfer/channel-1/ujunox
        bank: "ibc/8FB044422997A8A77891DE729EC28638DDE4C81A54398F68149A058AA9B74D9F",
        // full ibc route: transfer/channel-1/ujunox
        dex: "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0",
    }
}
define_currency!(Juno, JUNO, 6);

define_symbol! {
    MARS {
        // full ibc route: transfer/channel-0/transfer/channel-24/umars
        bank: "ibc/1CC042AD599E184C0F77DC5D89443C82F8A16B6E13DEC650A7A50A5D0AA330C3",
        // full ibc route: transfer/channel-24/umars
        dex: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9",
    }
}
define_currency!(Mars, MARS, 6);

pub(super) fn maybe_visit<M, V>(
    matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, Atom, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, Osmo, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Juno, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Mars, _>(matcher, visitor))
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

    use super::{Akt, Atom, Juno, Mars, Osmo, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Akt, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Juno, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Mars, LeaseGroup>();

        maybe_visit_on_ticker_err::<Lpn, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Lpn::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Akt, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Juno, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Mars, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, LeaseGroup>(Lpn::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Lpn::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::TICKER);
    }
}
