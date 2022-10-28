use finance::{coin::CoinDTO, currency::SymbolOwned, liability::Liability, percent::Percent};
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

pub mod borrow;
pub mod quote;

pub struct Quote<'r> {
    querier: QuerierWrapper<'r>,
    currency: SymbolOwned,
    downpayment: CoinDTO,
    oracle: OracleRef,
    liability: Liability,
    lease_interest_rate_margin: Percent,
}

pub struct Borrow {}
