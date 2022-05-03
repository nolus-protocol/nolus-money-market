use cosmwasm_std::{Api, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{interest::InterestPolicy, lease::Lease};

// TODO define it as type not alias
pub type Denom = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ApplicationForm {
    /// The customer who wants to open a lease.
    pub customer: String,
    /// Denomination of the currency this lease will be about.
    pub currency: String,
    pub liability: LiabilityPolicy,
    pub interest: InterestPolicyDTO,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LiabilityPolicy {
    /// The initial percentage of the amount due versus the locked collateral
    pub init_percent: u8,
    /// The healty percentage of the amount due versus the locked collateral
    pub healthy_percent: u8,
    /// The maximum percentage of the amount due versus the locked collateral
    pub max_percent: u8,
    /// At what time cadence to recalculate the liability
    pub recalc_secs: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// The value remains intact.
pub struct InterestPolicyDTO {
    /// The delta added on top of the LPP Loan interest rate.
    ///
    /// The amount, a part of any payment, goes to the Profit contract.
    pub annual_margin_interest_permille: u8,
    /// The Liquidity Provider Pool, LPP, that lends the necessary amount for this lease.
    pub lpp: String,
    /// How long is a period for which the interest is due
    pub interest_due_period_secs: u32,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    pub grace_period_secs: u32,
}

impl InterestPolicyDTO {
    pub fn into(self, api: &dyn Api) -> StdResult<InterestPolicy> {
        let lpp = api.addr_validate(&self.lpp)?;
        Ok(InterestPolicy::new(
            self.annual_margin_interest_permille,
            lpp,
            self.interest_due_period_secs,
            self.grace_period_secs,
        ))
    }
}

impl ApplicationForm {
    pub fn into(self, api: &dyn Api) -> StdResult<Lease> {
        let customer = api.addr_validate(&self.customer)?;
        Ok(Lease::new(
            customer,
            self.currency,
            self.liability,
            self.interest.into(api)?,
        ))
    }
}
