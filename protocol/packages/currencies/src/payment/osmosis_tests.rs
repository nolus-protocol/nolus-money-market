#![cfg(all(test, feature = "osmosis-osmosis-usdc_axelar"))]

use currency::Currency;

use crate::{
    lease::osmosis::{Atom, Osmo, StAtom, StOsmo, Wbtc, Weth},
    lpn::Lpn,
    native::osmosis::Nls,
    payment::PaymentGroup,
    test_impl::{
        maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl, maybe_visit_on_ticker_err,
        maybe_visit_on_ticker_impl,
    },
};

#[test]
fn maybe_visit_on_ticker() {
    maybe_visit_on_ticker_impl::<Atom, PaymentGroup>();
    maybe_visit_on_ticker_impl::<StAtom, PaymentGroup>();
    maybe_visit_on_ticker_impl::<Osmo, PaymentGroup>();
    maybe_visit_on_ticker_impl::<StOsmo, PaymentGroup>();
    maybe_visit_on_ticker_impl::<Weth, PaymentGroup>();
    maybe_visit_on_ticker_impl::<Wbtc, PaymentGroup>();
    maybe_visit_on_ticker_impl::<Lpn, PaymentGroup>();
    maybe_visit_on_ticker_impl::<Nls, PaymentGroup>();
    maybe_visit_on_ticker_err::<Nls, PaymentGroup>(Nls::BANK_SYMBOL);
    maybe_visit_on_ticker_err::<Atom, PaymentGroup>(Atom::BANK_SYMBOL);
    maybe_visit_on_ticker_err::<Lpn, PaymentGroup>(Nls::BANK_SYMBOL);
    maybe_visit_on_ticker_err::<Lpn, PaymentGroup>(Lpn::BANK_SYMBOL);
    maybe_visit_on_ticker_err::<Osmo, PaymentGroup>(Atom::BANK_SYMBOL);
    maybe_visit_on_ticker_err::<Osmo, PaymentGroup>(Osmo::BANK_SYMBOL);
}

#[test]
fn maybe_visit_on_bank_symbol() {
    maybe_visit_on_bank_symbol_impl::<Atom, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<StAtom, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<Osmo, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<StOsmo, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<Weth, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<Wbtc, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<Lpn, PaymentGroup>();
    maybe_visit_on_bank_symbol_impl::<Nls, PaymentGroup>();
    maybe_visit_on_bank_symbol_err::<Nls, PaymentGroup>(Nls::TICKER);
    maybe_visit_on_bank_symbol_err::<Atom, PaymentGroup>(Atom::TICKER);
    maybe_visit_on_bank_symbol_err::<Lpn, PaymentGroup>(Nls::TICKER);
    maybe_visit_on_bank_symbol_err::<Lpn, PaymentGroup>(Lpn::TICKER);
}
