use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, liability::Liability, percent::Percent};
use sdk::schemars::{self, JsonSchema};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: String,
    /// Symbol of the currency this lease will be about.
    pub currency: SymbolOwned,
    /// Liability parameters
    pub liability: Liability,
    pub loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    pub time_alarms: String,
    /// The oracle contract that sends market price alerts to the lease
    pub market_price_oracle: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    pub interest_due_period: Duration,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    pub grace_period: Duration,
    /// The Profit contract to which the margin interest is sent.
    pub profit: String,
}
