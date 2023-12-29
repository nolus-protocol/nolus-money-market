use currency::Group;
use finance::price::dto::PriceDTO;
use serde::Deserialize;

use super::{Alarm as ValidatedAlarm, AlarmError};

/// Brings invariant checking as a step in deserializing an Alarm
#[derive(Deserialize)]
pub(super) struct Alarm<G, LpnG>
where
    G: Group,
    LpnG: Group,
{
    below: PriceDTO<G, LpnG>,
    above: Option<PriceDTO<G, LpnG>>,
}

impl<G, LpnG> TryFrom<Alarm<G, LpnG>> for ValidatedAlarm<G, LpnG>
where
    G: Group,
    LpnG: Group,
{
    type Error = AlarmError;

    fn try_from(dto: Alarm<G, LpnG>) -> Result<Self, Self::Error> {
        let res = Self {
            below: dto.below,
            above: dto.above,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
