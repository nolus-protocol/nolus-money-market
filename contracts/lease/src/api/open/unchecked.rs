use serde::Deserialize;

use finance::{duration::Duration, liability::Liability};

use crate::{api::LpnCoin, error::ContractError};

use super::InterestPaymentSpec as ValidatedInterestPaymentSpec;
use super::PositionSpecDTO as ValidatedPositionSpec;

/// Brings invariant checking as a step in deserializing a InterestPaymentSpec
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
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
        res.invariant_held().map(|_| res)
    }
}

/// Brings invariant checking as a step in deserializing a PositionSpecDTO
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(super) struct PositionSpecDTO {
    liability: Liability,
    min_asset: LpnCoin,
    min_sell_asset: LpnCoin,
}

impl TryFrom<PositionSpecDTO> for ValidatedPositionSpec {
    type Error = ContractError;

    fn try_from(value: PositionSpecDTO) -> Result<Self, Self::Error> {
        let res = Self {
            liability: value.liability,
            min_asset: value.min_asset,
            min_sell_asset: value.min_sell_asset,
        };
        res.invariant_held().map(|_| res)
    }
}
