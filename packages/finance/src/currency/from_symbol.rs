use crate::error::Error;

use super::{Currency, Symbol};

pub trait SingleVisitor<C> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit_on_bank_symbol<C, V>(bank_symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    V: SingleVisitor<C>,
    C: Currency,
    Error: Into<V::Error>,
{
    maybe_visit_on_bank_symbol(bank_symbol, visitor)
        .unwrap_or_else(|_| Err(Error::unexpected_bank_symbol::<_, C>(bank_symbol).into()))
}

pub type MaybeVisitResult<C, V> =
    Result<Result<<V as SingleVisitor<C>>::Output, <V as SingleVisitor<C>>::Error>, V>;

pub fn maybe_visit_on_ticker<C, V>(ticker: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    C: Currency,
    V: SingleVisitor<C>,
{
    maybe_visit_impl(ticker, C::TICKER, visitor)
}

pub fn maybe_visit_on_bank_symbol<C, V>(bank_symbol: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    V: SingleVisitor<C>,
    C: Currency,
{
    maybe_visit_impl(bank_symbol, C::BANK_SYMBOL, visitor)
}

fn maybe_visit_impl<C, V>(symbol: Symbol, symbol_exp: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    V: SingleVisitor<C>,
    C: Currency,
{
    if symbol == symbol_exp {
        Ok(visitor.on())
    } else {
        Err(visitor)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        currency::Currency,
        error::Error,
        test::{
            currency::{Nls, Usdc},
            visitor::{Expect, ExpectUnknownCurrency},
        },
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
