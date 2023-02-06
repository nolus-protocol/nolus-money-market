use currency::{lpn::Lpns, native::Native};
use finance::coin::CoinDTO;
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, Storage, Timestamp};

use crate::state::Config;

mod dispatch;

pub struct Dispatch<'a> {
    storage: &'a dyn Storage,
    last_dispatch: Timestamp,
    oracle_ref: OracleRef,
    config: Config,
    block_time: Timestamp,
    querier: QuerierWrapper<'a>,
}

pub struct Result {
    pub batch: Batch,
    pub receipt: Receipt,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Receipt {
    pub in_stable: CoinDTO<Lpns>,
    pub in_nls: CoinDTO<Native>,
}
