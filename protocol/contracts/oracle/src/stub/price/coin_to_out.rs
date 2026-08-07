use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO, WithCoin};
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

use super::{Result, convert};

/// A transformer of an amount in any input currency into its equivalent in the output currency
///
/// Encapsulates the price source and the conversion route to the output currency.
pub trait CoinToOut<InG>
where
    InG: Group,
{
    /// The output currency
    type OutC: CurrencyDef;

    fn to_out(&self, input: &CoinDTO<InG>, querier: QuerierWrapper<'_>)
    -> Result<Coin<Self::OutC>>;
}

/// Convert into the quote currency of the price source
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
#[serde(transparent)]
pub struct ToQuote<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle: OracleRef<QuoteC, QuoteG>,
}

impl<QuoteC, QuoteG> ToQuote<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub const fn new(oracle: OracleRef<QuoteC, QuoteG>) -> Self {
        Self { oracle }
    }
}

impl<InG, QuoteC, QuoteG> CoinToOut<InG> for ToQuote<QuoteC, QuoteG>
where
    InG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG> + MemberOf<InG::TopG>,
    QuoteG: Group,
{
    type OutC = QuoteC;

    fn to_out(
        &self,
        input: &CoinDTO<InG>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Coin<Self::OutC>> {
        struct InCoinResolve<'querier, InG, QuoteC, QuoteG>
        where
            QuoteC: Currency + MemberOf<QuoteG>,
            QuoteG: Group,
        {
            oracle: OracleRef<QuoteC, QuoteG>,
            querier: QuerierWrapper<'querier>,
            _in_g: PhantomData<InG>,
        }

        impl<InG, QuoteC, QuoteG> WithCoin<InG> for InCoinResolve<'_, InG, QuoteC, QuoteG>
        where
            InG: Group,
            QuoteC: CurrencyDef,
            QuoteC::Group: MemberOf<QuoteG> + MemberOf<InG::TopG>,
            QuoteG: Group,
        {
            type Outcome = Result<Coin<QuoteC>>;

            fn on<C>(self, input: Coin<C>) -> Self::Outcome
            where
                C: CurrencyDef,
                C::Group: MemberOf<InG> + MemberOf<<InG as Group>::TopG>,
            {
                convert::to_quote::<C, InG, QuoteC, QuoteG>(self.oracle, input, self.querier)
            }
        }

        input.with_coin(InCoinResolve {
            oracle: self.oracle.clone(),
            querier,
            _in_g: PhantomData::<InG>,
        })
    }
}
