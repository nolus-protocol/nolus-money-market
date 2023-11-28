use serde::Deserialize;

use currency::{visit_any_on_tickers, AnyVisitorPair, AnyVisitorPairResult, Group};

use crate::{coin::CoinDTO, error::Error};

use super::PriceDTO as ValidatedDTO;

/// Brings invariant checking as a step in deserializing a PriceDTO
#[derive(Deserialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub(super) struct PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<QuoteG>,
}

impl<G, QuoteG> TryFrom<PriceDTO<G, QuoteG>> for ValidatedDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(dto: PriceDTO<G, QuoteG>) -> Result<Self, Self::Error> {
        visit_any_on_tickers::<G, QuoteG, _>(
            dto.amount.ticker(),
            dto.amount_quote.ticker(),
            VisitorImpl,
        )?;

        let res = Self {
            amount: dto.amount,
            amount_quote: dto.amount_quote,
        };

        res.invariant_held()?;

        Ok(res)
    }
}

struct VisitorImpl;

impl AnyVisitorPair for VisitorImpl {
    type Output = ();

    type Error = Error;

    fn on<C1, C2>(self) -> AnyVisitorPairResult<Self> {
        Ok(())
    }
}
