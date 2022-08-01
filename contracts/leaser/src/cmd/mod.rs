use finance::{coin::CoinDTO, liability::Liability, percent::Percent};

pub mod borrow;
pub mod quote;

pub struct Quote {
    downpayment: CoinDTO,
    liability: Liability,
    lease_interest_rate_margin: Percent,
}

pub struct Borrow {}
