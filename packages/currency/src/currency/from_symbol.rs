use crate::{error::Error, SymbolSlice};

use super::{
    group,
    matcher::{BankSymbolMatcher, TickerMatcher},
    Currency,
};

pub trait SingleVisitor<C> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit_on_bank_symbol<C, V>(
    bank_symbol: &SymbolSlice,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    V: SingleVisitor<C>,
    C: Currency,
    Error: Into<V::Error>,
{
    group::maybe_visit(BankSymbolMatcher, bank_symbol, visitor)
        .unwrap_or_else(|_| Err(Error::unexpected_bank_symbol::<_, C>(bank_symbol).into()))
}

pub type MaybeVisitResult<C, V> =
    Result<Result<<V as SingleVisitor<C>>::Output, <V as SingleVisitor<C>>::Error>, V>;

pub fn maybe_visit_on_ticker<C, V>(ticker: &SymbolSlice, visitor: V) -> MaybeVisitResult<C, V>
where
    C: Currency,
    V: SingleVisitor<C>,
{
    group::maybe_visit(TickerMatcher, ticker, visitor)
}

#[cfg(test)]
mod test {
    use crate::test::{Nls, Usdc};
    use crate::{
        currency::Currency,
        error::Error,
        test::visitor::{Expect, ExpectUnknownCurrency},
    };

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<Usdc>::default();
        assert_eq!(
            super::visit_on_bank_symbol(Usdc::BANK_SYMBOL, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<Nls>::default();
        assert_eq!(
            super::visit_on_bank_symbol(Nls::BANK_SYMBOL, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit_on_bank_symbol::<Nls, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::unexpected_bank_symbol::<_, Nls>(DENOM,)),
        );
    }
}
