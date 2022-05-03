use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
/// The value remains intact.
pub struct Loan {
    annual_margin_interest_permille: u8,
    lpp: Addr,
    interest_due_period_secs: u32,
    grace_period_secs: u32,
}

impl Loan {
    pub fn new(
        annual_margin_interest_permille: u8,
        lpp: Addr,
        interest_due_period_secs: u32,
        grace_period_secs: u32,
    ) -> Self {
        Self {
            annual_margin_interest_permille,
            lpp,
            interest_due_period_secs,
            grace_period_secs,
        }
    }
}
