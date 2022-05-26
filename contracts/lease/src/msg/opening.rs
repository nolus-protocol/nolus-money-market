use finance::{liability::Liability, percent::Percent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO define it as type not alias
pub type Denom = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: String,
    /// Denomination of the currency this lease will be about.
    pub currency: String,
    pub liability: Liability,
    pub loan: LoanForm,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "loan")]
/// The value remains intact.
pub struct LoanForm {
    /// The delta added on top of the LPP Loan interest rate.
    ///
    /// The amount, a part of any payment, goes to the Profit contract.
    pub annual_margin_interest: Percent,
    /// The Liquidity Provider Pool, LPP, that lends the necessary amount for this lease.
    pub lpp: String,
    /// How long is a period for which the interest is due
    pub interest_due_period_secs: u32,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    pub grace_period_secs: u32,
}
