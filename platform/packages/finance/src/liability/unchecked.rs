use serde::Deserialize;

use crate::{duration::Duration, error::Error, percent::Percent100};

use super::Liability as ValidatedLiability;

/// Brings invariant checking as a step in deserializing a Liability
#[derive(Deserialize)]
pub(super) struct Liability {
    initial: Percent100,
    healthy: Percent100,
    first_liq_warn: Percent100,
    second_liq_warn: Percent100,
    third_liq_warn: Percent100,
    max: Percent100,
    recalc_time: Duration,
}

impl TryFrom<Liability> for ValidatedLiability {
    type Error = Error;

    fn try_from(dto: Liability) -> Result<Self, Self::Error> {
        let res = Self {
            initial: dto.initial,
            healthy: dto.healthy,
            first_liq_warn: dto.first_liq_warn,
            second_liq_warn: dto.second_liq_warn,
            third_liq_warn: dto.third_liq_warn,
            max: dto.max,
            recalc_time: dto.recalc_time,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
