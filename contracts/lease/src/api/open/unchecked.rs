use crate::error::ContractError;
use finance::duration::Duration;
use serde::Deserialize;

use super::InterestPaymentSpec as ValidatedInterestPaymentSpec;

/// Brings invariant checking as a step in deserializing a InterestPaymentSpec
#[derive(Deserialize)]
pub(super) struct InterestPaymentSpec {
    due_period: Duration,
    grace_period: Duration,
}

impl TryFrom<InterestPaymentSpec> for ValidatedInterestPaymentSpec {
    type Error = ContractError;

    fn try_from(dto: InterestPaymentSpec) -> Result<Self, Self::Error> {
        let res = Self {
            due_period: dto.due_period,
            grace_period: dto.grace_period,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
