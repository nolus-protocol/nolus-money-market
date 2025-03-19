use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::error::Result;

pub trait Enterable {
    fn enter(&self, now: Timestamp, querier: QuerierWrapper<'_>) -> Result<Batch>;
}
