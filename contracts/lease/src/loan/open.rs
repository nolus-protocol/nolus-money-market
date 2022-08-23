use cosmwasm_std::Addr;

use finance::{
    currency::Currency,
    percent::Percent,
    coin::Coin
};
use platform::batch::Batch;

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub annual_interest_rate: Percent,
    pub borrowed: Coin<Lpn>,
    pub loan_pool_id: Addr,
}
