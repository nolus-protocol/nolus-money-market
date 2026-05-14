use finance::instant::Instant;
use platform::batch::Batch;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::error::Result;

pub trait Enterable {
    fn enter(&self, now: Instant, querier: QuerierWrapper<'_>) -> Result<Batch>;
}
