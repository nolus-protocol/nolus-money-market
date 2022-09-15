use serde::{de::DeserializeOwned, Serialize};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency},
    price::Price,
};

use super::{CVisitor, PriceDTO, WithPrice};

pub fn execute<Cmd>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithPrice,
{
    visit_any(
        &price.amount.symbol().clone(),
        CVisitor {
            price_dto: price,
            cmd,
        },
    )
}
