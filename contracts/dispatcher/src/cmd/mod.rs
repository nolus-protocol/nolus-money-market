use cosmwasm_std::Timestamp;
use finance::{
    coin::Coin,
    currency::{Currency, Nls},
};
use marketprice::storage::Price;
use platform::batch::Batch;

use crate::state::Config;

mod dispatch;
mod dispatcher;
mod get_price;

pub struct GetPrice {}

pub struct Dispatch {
    last_dispatch: Timestamp,
    price: Price,
    config: Config,
    block_time: Timestamp,
}

pub struct Result<C>
where
    C: Currency,
{
    pub batch: Batch,
    pub receipt: Receipt<C>,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Receipt<C>
where
    C: Currency,
{
    in_stable: Coin<C>,
    in_nls: Coin<Nls>,
}
