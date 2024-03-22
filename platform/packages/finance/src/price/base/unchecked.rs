use currency::Group;
use serde::Deserialize;

use crate::{
    coin::{Coin, CoinDTO},
    error::Error,
};

use crate::price::base::BasePrice as ValidatedBasePrice;

/// Brings invariant checking as a step in deserializing a BasePrice
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: ?Sized,
{
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
}

impl<BaseG, QuoteC> TryFrom<BasePrice<BaseG, QuoteC>> for ValidatedBasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: ?Sized,
{
    type Error = Error;

    fn try_from(value: BasePrice<BaseG, QuoteC>) -> Result<Self, Self::Error> {
        let res = Self {
            amount: value.amount,
            amount_quote: value.amount_quote,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
