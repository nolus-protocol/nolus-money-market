use currency::{Currency, Group};
use finance::price::base::BasePrice;
use serde::Deserialize;

use super::{Alarm as ValidatedAlarm, Error};

/// Brings invariant checking as a step in deserializing an Alarm
#[derive(Deserialize)]
pub(super) struct Alarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    below: BasePrice<G, Lpn, Lpns>,
    above: Option<BasePrice<G, Lpn, Lpns>>,
}

impl<G, Lpn, Lpns> TryFrom<Alarm<G, Lpn, Lpns>> for ValidatedAlarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    type Error = Error;

    fn try_from(dto: Alarm<G, Lpn, Lpns>) -> Result<Self, Self::Error> {
        let res = Self {
            below: dto.below,
            above: dto.above,
        };
        res.invariant_held()?;
        Ok(res)
    }
}
