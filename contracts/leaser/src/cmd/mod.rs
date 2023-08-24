use currency::SymbolOwned;
use finance::{liability::LiabilityDTO, percent::Percent};
use lease::api::DownpaymentCoin;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

pub mod borrow;
pub mod quote;

pub struct Quote<'r> {
    querier: QuerierWrapper<'r>,
    lease_asset: SymbolOwned,
    downpayment: DownpaymentCoin,
    oracle: OracleRef,
    liability: LiabilityDTO,
    lease_interest_rate_margin: Percent,
    max_ltd: Option<Percent>,
}

pub struct Borrow {}
