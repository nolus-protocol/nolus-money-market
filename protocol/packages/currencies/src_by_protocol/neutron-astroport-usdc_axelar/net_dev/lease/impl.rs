use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars;

use crate::{define_currency, LeaseGroup};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_currency!(
    Atom,
    "ATOM",
    "ibc/2E935FE009C5499B9EF05DA9DBA83E0132F3D1CB99409068579ECC1A0B02A3D6", // transfer/channel-3/transfer/channel-1/uatom
    "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9", // transfer/channel-1/uatom
    LeaseGroup,
    6
);

define_currency!(
    Ntrn,
    "NTRN",
    "ibc/7C1B575B45FDB34A291FDBFC1CDC01A2196D4BDD11C8C1930F2576D310B31119", // transfer/channel-3/untrn
    "untrn",
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
        .or_else(|visitor| maybe_visit::<_, Ntrn, TopG, _>(matcher, visitor))
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
            payment::PaymentGroup,
        },
    };

    use super::{Atom, Ntrn};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Ntrn, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Atom, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Ntrn, PaymentGroup>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::bank());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::ticker());
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::dex());
        maybe_visit_on_ticker_err::<Ntrn, LeaseGroup>(Ntrn::bank());
        maybe_visit_on_ticker_err::<Ntrn, LeaseGroup>(Ntrn::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Ntrn, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Atom, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Ntrn, PaymentGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::dex());
        maybe_visit_on_bank_symbol_err::<Ntrn, LeaseGroup>(Ntrn::dex());
    }
}
