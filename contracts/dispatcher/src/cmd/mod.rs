use currency::native::Nls;
use finance::{coin::Coin, currency::Currency};
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, QuerierWrapper, Timestamp};

use crate::state::Config;

mod dispatch;

pub struct Dispatch<'a> {
    deps: Deps<'a>,
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
