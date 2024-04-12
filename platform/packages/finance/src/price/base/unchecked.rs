use serde::Deserialize;

use currency::{Currency, Group};

use crate::coin::Coin;
use crate::{coin::CoinDTO, error::Error};

use crate::price::base::BasePrice as ValidatedBasePrice;

/// Brings invariant checking as a step in deserializing a BasePrice
#[derive(Deserialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub(super) struct BasePrice<BaseG, QuoteG>
where
    BaseG: Group,
    QuoteG: Group,
{
    amount: CoinDTO<BaseG>,
    amount_quote: CoinDTO<QuoteG>,
}

impl<BaseG, QuoteG, QuoteC> TryFrom<BasePrice<BaseG, QuoteG>>
    for ValidatedBasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + ?Sized,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(value: BasePrice<BaseG, QuoteG>) -> Result<Self, Self::Error> {
        Coin::<QuoteC>::try_from(value.amount_quote)
            .and_then(|amount_quote| ValidatedBasePrice::new_checked(value.amount, amount_quote))
    }
}
