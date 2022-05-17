use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::error::ContractResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
/// The value remains intact.
pub struct Loan<L> {
    annual_margin_interest_permille: u8,
    lpp: L,
    interest_due_period_secs: u32,
    grace_period_secs: u32,
}

impl<L> Loan<L>
where
    L: Lpp,
{
    pub fn open(
        lpp: L,
        annual_margin_interest_permille: u8,
        interest_due_period_secs: u32,
        grace_period_secs: u32,
    ) -> ContractResult<Self> {
        // TODO query lpp about its denom
        // const LPP_DENOM: &str = "UST";

        // check them out cw_utils::Duration, cw_utils::NativeBalance
        Ok(Self {
            annual_margin_interest_permille,
            lpp,
            interest_due_period_secs,
            grace_period_secs,
        })
    }
}
