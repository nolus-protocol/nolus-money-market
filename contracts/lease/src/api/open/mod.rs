use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, liability::Liability, percent::Percent};
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::{error::ContractError, error::ContractResult};

use super::dex::ConnectionParams;

mod unchecked;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseContract {
    /// An application form for opening a new lease
    pub form: NewLeaseForm,
    /// Connection parameters of a Dex capable to perform currency swaps
    pub dex: ConnectionParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: Addr,
    /// Ticker of the currency this lease will be about.
    pub currency: SymbolOwned,
    /// Maximum Loan-to-Value percentage of the new lease, optional.
    pub max_ltv: Option<Percent>,
    /// Liability parameters
    pub liability: Liability,
    /// Loan parameters
    pub loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    pub time_alarms: Addr,
    /// The oracle contract that sends market price alerts to the lease
    pub market_price_oracle: Addr,
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
    pub lpp: Addr,
    /// Interest repayment parameters
    pub interest_payment: InterestPaymentSpec,
    /// The Profit contract to which the margin interest is sent.
    pub profit: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(try_from = "unchecked::InterestPaymentSpec")]
pub struct InterestPaymentSpec {
    /// How long is a period for which the interest is due
    due_period: Duration,
    /// How long after the due period ends the interest may be paid before initiating a liquidation
    grace_period: Duration,
}

impl InterestPaymentSpec {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(due_period: Duration, grace_period: Duration) -> Self {
        let res = Self {
            due_period,
            grace_period,
        };
        debug_assert_eq!(res.invariant_held(), Ok(()));
        res
    }

    pub fn grace_period(&self) -> Duration {
        self.grace_period
    }

    pub fn due_period(&self) -> Duration {
        self.due_period
    }

    fn invariant_held(&self) -> ContractResult<()> {
        ContractError::broken_invariant_if::<InterestPaymentSpec>(
            self.due_period == Duration::default(),
            "The interest due period should be with non-zero length",
        )
        .and_then(|_| {
            ContractError::broken_invariant_if::<InterestPaymentSpec>(
                self.due_period <= self.grace_period,
                "The interest due period should be longer than grace period to avoid overlapping",
            )
        })
    }
}

#[cfg(test)]
mod test_invariant {
    use finance::duration::Duration;
    use sdk::cosmwasm_std::{from_slice, StdError};

    use super::InterestPaymentSpec;

    #[test]
    #[should_panic = "non-zero length"]
    fn due_period_zero() {
        new_invalid(Duration::default(), Duration::from_hours(2));
    }

    #[test]
    fn due_period_zero_json() {
        let r = from_slice(br#"{"due_period": 0, "grace_period": 10}"#);
        assert_err(r, "non-zero length");
    }

    #[test]
    #[should_panic = "should be longer than"]
    fn due_shorter_than_grace() {
        new_invalid(
            Duration::from_days(100),
            Duration::from_days(100) + Duration::from_nanos(1),
        );
    }

    #[test]
    fn due_shorter_than_grace_json() {
        let r = from_slice(br#"{"due_period": 9, "grace_period": 10}"#);
        assert_err(r, "should be longer than");
    }

    #[test]
    #[should_panic = "should be longer than"]
    fn due_equal_to_grace() {
        new_invalid(Duration::from_days(100), Duration::from_days(100));
    }

    #[test]
    fn due_equal_to_grace_json() {
        let r = from_slice(br#"{"due_period": 10, "grace_period": 10}"#);
        assert_err(r, "should be longer than");
    }

    fn new_invalid(due_period: Duration, grace_period: Duration) {
        let _p = InterestPaymentSpec::new(due_period, grace_period);
        #[cfg(not(debug_assertions))]
        {
            _p.invariant_held().expect("should have returned an error");
        }
    }

    fn assert_err(r: Result<InterestPaymentSpec, StdError>, msg: &str) {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("InterestPaymentSpec") && real_msg.contains(msg)
        ));
    }
}
