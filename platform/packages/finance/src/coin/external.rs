use serde::Deserialize;

use currency::{CurrencyDef, SymbolOwned, SymbolRef};

#[cfg(test)]
use crate::coin::Amount;
use crate::{coin::Coin as GenericCoin, error::Result};

/// Designed to allow coercion of coins of external or unknown currencies,
/// into coins of a known currency, for example, [`Coin<Stable>`].
///
/// The coercion acts on deserialization. On the wire, all coins must have the same representation
/// carrying some amount and the currency ticker. If the exact currency is known
/// on the receiving side at compile time, then it is preferred to deserialize into [`GenericCoin<C>`].
///
/// If the currency is known on the receiving side as a member of a set of currencies,
/// and not which one exactly, then it is preferred to be deserialized into [`crate::coin::CoinDTO<G>`].
///
/// If the currency is unknown on the receiving side, then it may be 'coerced', or 'mapped' to
/// a known currency by deserializing into [Coin<C>], where the type parameter 'C' is the currency
/// into which the coin will be coerced into.
///
/// Therefore this type should be used only in data transfer use cases involving external,
/// hence unknown currencies, For example, querying Lpp stable balances of protocols by
/// the rewards calculator on the platform.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(deserialize = "")
)]
pub struct Coin<C> {
    #[serde(flatten)]
    coin: GenericCoin<C>,

    /// Must be named as [`CoinDTO<G>`] `currency` on the wire
    ticker: SymbolOwned,
}

impl<C> Coin<C>
where
    C: CurrencyDef,
{
    pub fn try_coerce(self, expected_ticker: SymbolRef<'_>) -> Result<GenericCoin<C>> {
        currency::expect_exact_received::<C, C::Group>(expected_ticker, &self.ticker)
            .map_err(Into::into)
            .map(|()| self.coin)
    }

    #[cfg(test)]
    pub(crate) fn test_new<T>(amount: Amount, received_ticker: T) -> Self
    where
        T: Into<SymbolOwned>,
    {
        Self {
            coin: GenericCoin::new(amount),
            ticker: received_ticker.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use currency::{CurrencyDef, platform::Nls, test::SuperGroupTestC1};
    use platform::tests as platform_tests;

    use crate::coin::{Amount, Coin as GenericCoin, CoinDTO};

    use super::Coin;

    type ExternalC = SuperGroupTestC1;

    const AMOUNT: Amount = 676576;

    #[test]
    fn dto_to_type() {
        assert_eq!(
            Ok(Coin::<Nls>::test_new(AMOUNT, ExternalC::ticker())),
            platform_tests::ser_de(&CoinDTO::from(GenericCoin::<ExternalC>::new(AMOUNT))),
        );
    }

    #[test]
    fn coerce() {
        let external_currency_ticker = ExternalC::ticker();
        let coin = Coin::<Nls>::test_new(AMOUNT, external_currency_ticker);
        assert_eq!(
            Ok(GenericCoin::new(AMOUNT)),
            coin.clone().try_coerce(external_currency_ticker)
        );
        assert!(coin.try_coerce("other ticker").is_err());
    }
}
