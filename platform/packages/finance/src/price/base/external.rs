use serde::Deserialize;

#[cfg(test)]
use currency::SymbolOwned;
use currency::{CurrencyDef, Group, MemberOf, SymbolRef};

#[cfg(test)]
use crate::coin::Amount;
use crate::{
    coin::{CoinDTO, ExternalCoinDTO},
    error::Result,
};

use super::BasePrice as GenericPrice;

/// Designed to allow coercion of prices with external or unknown quote currencies,
/// into prices with a known quote currency, for example, [`Price<PlatformGroup, Stable, PlatformGroup>`].
///
/// The coercion acts on deserialization. On the wire, all prices must have the same representation
/// carrying the amount and the quote amount. If the exact quote currency is known
/// on the receiving side at compile time, then it is preferred to deserialize into
/// [`GenericPrice<BaseG, QuoteC, QuoteG>`].
///
/// If the quote currency is known on the receiving side as a member of a set of currencies,
/// and not which one exactly, then it is preferred to be deserialized into [`crate::price::dto::PriceDTO<G>`].
///
/// If the quote currency is unknown on the receiving side, then it may be 'coerced', or 'mapped' to
/// a known currency by deserializing into [Price<BaseG, QuoteC, QuoteG>], where the type parameter
/// 'QuoteC' is the quote currency into which the price will be coerced into.
///
/// Therefore this type should be used only in data transfer use cases involving external,
/// hence unknown quote currencies, For example, querying protocols for the Nls price by
/// the rewards calculator on the platform.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(deserialize = "")
)]
pub struct Price<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: CurrencyDef,
{
    amount: CoinDTO<BaseG>,
    amount_quote: ExternalCoinDTO<QuoteC>,
}

impl<BaseG, QuoteC> Price<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: CurrencyDef,
{
    pub fn try_coerce<QuoteG>(
        self,
        expected_ticker: SymbolRef<'_>,
    ) -> Result<GenericPrice<BaseG, QuoteC, QuoteG>>
    where
        QuoteC::Group: MemberOf<QuoteG>,
        QuoteG: Group,
    {
        self.amount_quote
            .try_coerce(expected_ticker)
            .and_then(|amount_quote| GenericPrice::new_checked(self.amount, amount_quote))
    }

    #[cfg(test)]
    fn test_new<C, T>(amount: Amount, amount_quote: Amount, received_ticker: T) -> Self
    where
        C: CurrencyDef,
        C::Group: MemberOf<BaseG>,
        T: Into<SymbolOwned>,
    {
        use crate::coin::Coin;

        Self {
            amount: Coin::<C>::new(amount).into(),
            amount_quote: ExternalCoinDTO::test_new(amount_quote, received_ticker),
        }
    }
}

#[cfg(test)]
mod test {
    use currency::{
        CurrencyDef,
        platform::{Nls, PlatformGroup, Stable},
        test::SuperGroupTestC5,
    };
    use platform::tests as platform_tests;

    use crate::{
        coin::{Amount, Coin as GenericCoin},
        price::base::BasePrice,
    };

    use super::Price;

    type ExternalQuoteC = SuperGroupTestC5;
    type BaseC = Nls;

    const AMOUNT: Amount = 676576;
    const AMOUNT_QUOTE: Amount = 14;

    #[test]
    fn dto_to_type() {
        assert_eq!(
            Ok(Price::<PlatformGroup, Stable>::test_new::<BaseC, _>(
                AMOUNT,
                AMOUNT_QUOTE,
                ExternalQuoteC::ticker()
            )),
            platform_tests::ser_de(&BasePrice::new(
                GenericCoin::<BaseC>::new(AMOUNT).into(),
                GenericCoin::<ExternalQuoteC>::new(AMOUNT_QUOTE)
            )),
        );
    }

    #[test]
    fn coerce() {
        let external_currency_ticker = ExternalQuoteC::ticker();
        let price = Price::<PlatformGroup, Stable>::test_new::<BaseC, _>(
            AMOUNT,
            AMOUNT_QUOTE,
            external_currency_ticker,
        );
        assert_eq!(
            Ok(BasePrice::new(
                GenericCoin::<BaseC>::new(AMOUNT).into(),
                GenericCoin::<Stable>::new(AMOUNT_QUOTE)
            )),
            price.clone().try_coerce(external_currency_ticker)
        );
        assert!(price.try_coerce("other ticker").is_err());
    }
}
