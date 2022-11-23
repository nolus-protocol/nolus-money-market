use serde::Deserialize;

use crate::SpotPrice;

use super::{errors::AlarmError, Alarm as ValidatedAlarm};

/// Brings invariant checking as a step in deserializing an Alarm
#[derive(Deserialize)]
pub(super) struct Alarm {
    below: SpotPrice,
    above: Option<SpotPrice>,
}

impl TryFrom<Alarm> for ValidatedAlarm {
    type Error = AlarmError;

    fn try_from(dto: Alarm) -> Result<Self, Self::Error> {
        let res = Self {
            below: dto.below,
            above: dto.above,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
