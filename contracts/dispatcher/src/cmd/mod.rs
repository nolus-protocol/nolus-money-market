use cosmwasm_std::{QuerierWrapper, Timestamp};
use finance::{
    coin::Coin,
    currency::{Currency, Nls},
};
use oracle::stub::OracleRef;
use platform::batch::Batch;
use serde::Serialize;

use crate::state::Config;

mod dispatch;
mod price_convert;

pub struct PriceConvert<Lpn>
where
    Lpn: Currency + Serialize,
{
    amount: Coin<Lpn>,
}

pub struct Dispatch<'a> {
    last_dispatch: Timestamp,
    oracle_ref: OracleRef,
    config: Config,
    block_time: Timestamp,
    querier: QuerierWrapper<'a>,
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
