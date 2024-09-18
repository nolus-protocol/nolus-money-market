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
pub(super) struct PriceDTO<G>
where
    G: Group,
{
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<G>,
}

impl<G> TryFrom<PriceDTO<G>> for ValidatedDTO<G>
where
    G: Group<TopG = G>,
{
    type Error = Error;

    fn try_from(dto: PriceDTO<G>) -> Result<Self, Self::Error> {
        Self::try_new(dto.amount, dto.amount_quote)
    }
}
