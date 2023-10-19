#[cfg(test)]
mod test {
    use crate::{
        dex::test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        dex::{
            lease::osmosis::{Atom, Osmo, StAtom, StOsmo, Wbtc, Weth},
            lpn::osmosis::Usdc,
            native::osmosis::Nls,
            payment::PaymentGroup,
        },
        Currency,
    };

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, PaymentGroup>();
        maybe_visit_on_ticker_impl::<StAtom, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Osmo, PaymentGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Weth, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Nls, PaymentGroup>();
        maybe_visit_on_ticker_err::<Nls, PaymentGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, PaymentGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Usdc::BANK_SYMBOL);
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
        maybe_visit_on_bank_symbol_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Nls, PaymentGroup>();
        maybe_visit_on_bank_symbol_err::<Nls, PaymentGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, PaymentGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Usdc::TICKER);
    }
}
