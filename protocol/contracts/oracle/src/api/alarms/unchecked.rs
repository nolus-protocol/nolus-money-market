use currency::{Currency, Group};
use finance::price::base::BasePrice;
use serde::Deserialize;

use super::{Alarm as ValidatedAlarm, AlarmError};

/// Brings invariant checking as a step in deserializing an Alarm
#[derive(Deserialize)]
pub(super) struct Alarm<G, Lpn>
where
    G: Group,
    Lpn: Currency,
{
    below: BasePrice<G, Lpn>,
    above: Option<BasePrice<G, Lpn>>,
}

impl<G, Lpn> TryFrom<Alarm<G, Lpn>> for ValidatedAlarm<G, Lpn>
where
    G: Group,
    Lpn: Currency,
{
    type Error = AlarmError;

    fn try_from(dto: Alarm<G, Lpn>) -> Result<Self, Self::Error> {
        let res = Self {
            below: dto.below,
            above: dto.above,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
