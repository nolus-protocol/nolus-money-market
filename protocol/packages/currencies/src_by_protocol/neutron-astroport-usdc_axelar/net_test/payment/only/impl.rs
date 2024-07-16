use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult};

pub(super) fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher,
    V: AnyVisitor,
{
    currency::visit_noone(visitor)
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        lpn::{Lpn, Lpns},
        native::Nls,
        payment::only::PaymentOnlyGroup,
        test_impl::{maybe_visit_on_bank_symbol_err, maybe_visit_on_ticker_err},
    };

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_err::<Nls, PaymentOnlyGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Nls, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Lpn, PaymentOnlyGroup>(Lpn::TICKER);
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Nls::TICKER);

        maybe_visit_on_ticker_err::<Nls, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Lpn, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Lpn, PaymentOnlyGroup>(Lpn::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_err::<Nls, PaymentOnlyGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Nls, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Lpn, PaymentOnlyGroup>(Lpn::TICKER);
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Nls::TICKER);

        maybe_visit_on_bank_symbol_err::<Nls, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Lpn, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Lpn, PaymentOnlyGroup>(Lpn::BANK_SYMBOL);
    }
}
