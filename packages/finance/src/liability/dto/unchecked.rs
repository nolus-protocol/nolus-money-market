use serde::Deserialize;

use crate::{duration::Duration, error::Error, percent::Percent};

use super::{LiabilityDTO as ValidatedDTO, LpnCoin};

/// Brings invariant checking as a step in deserializing a LiabilityDTO
#[derive(Deserialize)]
pub(super) struct LiabilityDTO {
    initial: Percent,
    healthy: Percent,
    first_liq_warn: Percent,
    second_liq_warn: Percent,
    third_liq_warn: Percent,
    max: Percent,
    min_liq_amount: LpnCoin,
    min_asset_amount: LpnCoin,
    recalc_time: Duration,
}

impl TryFrom<LiabilityDTO> for ValidatedDTO {
    type Error = Error;

    fn try_from(dto: LiabilityDTO) -> Result<Self, Self::Error> {
        let res = Self {
            initial: dto.initial,
            healthy: dto.healthy,
            first_liq_warn: dto.first_liq_warn,
            second_liq_warn: dto.second_liq_warn,
            third_liq_warn: dto.third_liq_warn,
            max: dto.max,
            min_liq_amount: dto.min_liq_amount,
            min_asset_amount: dto.min_asset_amount,
            recalc_time: dto.recalc_time,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
