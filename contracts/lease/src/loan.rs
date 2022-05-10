use cosmwasm_std::Coin;
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
        borrow: Coin,
        mut lpp: L,
        annual_margin_interest_permille: u8,
        interest_due_period_secs: u32,
        grace_period_secs: u32,
    ) -> ContractResult<Self> {
        // TODO query lpp about its denom
        // const LPP_DENOM: &str = "UST";

        // let borrow_amount = Coin::new(amount, denom)
        // lpp::msg::ExecuteMsg::Loan{}
        // check them out cw_utils::Duration, cw_utils::NativeBalance
        lpp.open_loan_async(borrow)?;
        Ok(Self {
            annual_margin_interest_permille,
            lpp,
            interest_due_period_secs,
            grace_period_secs,
        })
    }
}
