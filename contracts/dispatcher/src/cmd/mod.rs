use cosmwasm_std::{QuerierWrapper, Storage, Timestamp};
use finance::{
    coin::Coin,
    currency::{Currency, Nls},
};
use platform::batch::Batch;

use crate::state::Config;

mod dispatch;
mod dispatcher;

pub struct Dispatch<'a> {
    storage: &'a mut dyn Storage,
    querier: QuerierWrapper<'a>,
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
