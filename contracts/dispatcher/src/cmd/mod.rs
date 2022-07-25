use cosmwasm_std::{QuerierWrapper, Storage, Timestamp};

use crate::state::Config;

mod dispatch;
mod dispatcher;

pub struct Dispatch<'a> {
    storage: &'a mut dyn Storage,
    querier: QuerierWrapper<'a>,
    config: Config,
    block_time: Timestamp,
}
