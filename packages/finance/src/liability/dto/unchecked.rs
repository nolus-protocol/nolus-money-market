use serde::Deserialize;

use crate::{duration::Duration, error::Error, liability::invariant_held, percent::Percent};

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
    min_liquidation: LpnCoin,
    min_asset: LpnCoin,
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
            min_liquidation: dto.min_liquidation,
            min_asset: dto.min_asset,
            recalc_time: dto.recalc_time,
        };
        invariant_held(
            &res,
            res.initial,
            res.healthy,
            (res.first_liq_warn, res.second_liq_warn, res.third_liq_warn),
            res.max,
            res.recalc_time,
        )?;
        Ok(res)
    }
}
