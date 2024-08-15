use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, LeaseGroup};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_currency!(
    Atom,
    "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196", // transfer/channel-0/transfer/channel-12/uatom
    "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477", // transfer/channel-12/uatom
    LeaseGroup,
    6
);

define_currency!(
    Osmo,
    "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518", // transfer/channel-0/uosmo
    "uosmo",
    LeaseGroup,
    6
);

define_currency!(
    Weth,
    "ibc/98CD37B180F06F954AFC71804049BE6EEA2A3B0CCEA1F425D141245BCFFBBD33", // transfer/channel-0/transfer/channel-3/eth-wei
    "ibc/29320BE25C3BF64A2355344625410899C1EB164038E328531C36095B0AA8BBFC", // transfer/channel-3/eth-wei
    // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
    // although there is no pool WETH participates in
    LeaseGroup,
    18
);

define_currency!(
    Wbtc,
    "ibc/680E95D3CEA378B7302926B8A5892442F1F7DF78E22199AE248DCBADC9A0C1A2", // transfer/channel-0/transfer/channel-3/btc-satoshi
    "ibc/CEDA3AFF171E72ACB689B7B64E988C0077DA7D4BF157637FFBDEB688D205A473", // transfer/channel-3/btc-satoshi
    // channel-3 is the official channel with Axelar as per https://docs.axelar.dev/resources/testnet
    // although there is no denomination trace as per `osmosisd q ibc-transfer denom-trace`
    LeaseGroup,
    8
);

define_currency!(
    Akt,
    "ibc/1064EED4A8E99F9C1158680236D0C5C3EA6B8BB65C9F87DAC6BC759DD904D818", // transfer/channel-0/transfer/channel-73/uakt
    "ibc/7153C8C55DB988805FAC69E449B680A8BAAC15944B87CF210ADCD1A3A9542857", // transfer/channel-73/uakt
    LeaseGroup,
    6
);

define_currency!(
    Juno,
    "ibc/8FB044422997A8A77891DE729EC28638DDE4C81A54398F68149A058AA9B74D9F", // transfer/channel-0/transfer/channel-1/ujunox
    "ibc/8E2FEFCBD754FA3C97411F0126B9EC76191BAA1B3959CB73CECF396A4037BBF0", // transfer/channel-1/ujunox
    LeaseGroup,
    6
);

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    LeaseGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    use currency::maybe_visit_member as maybe_visit;
    maybe_visit::<_, Atom, TopG, _>(matcher, visitor)
        .or_else(|visitor| maybe_visit::<_, Osmo, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, TopG, _>(matcher, visitor))
        .or_else(|visitor| maybe_visit::<_, Juno, TopG, _>(matcher, visitor))
}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {
            lease::LeaseGroup,
            lpn::{Lpn, Lpns},
            native::Nls,
        },
    };

    use super::{Akt, Atom, Juno, Osmo, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Akt, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Juno, LeaseGroup>();

        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::dex());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::ticker());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Lpn::bank());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Akt, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Juno, LeaseGroup>();

        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::dex());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::ticker());
    }
}
