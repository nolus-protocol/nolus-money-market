use cosmwasm_std::Addr;
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    percent::Percent,
};
use platform::batch::Batch;

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub customer: Addr,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
    pub currency: SymbolOwned,
    pub loan_pool_id: Addr,
    pub loan_amount: Coin<Lpn>,
}
