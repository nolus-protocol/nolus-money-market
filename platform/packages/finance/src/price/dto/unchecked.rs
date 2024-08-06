use serde::Deserialize;

use currency::Group;

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
        Self::try_new(dto.amount, dto.amount_quote)
    }
}
