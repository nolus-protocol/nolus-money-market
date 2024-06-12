use currency::{Currency, Group};
use finance::price::dto::PriceDTO;
use serde::{Deserialize, Serialize};

use super::{Alarm as ValidatedAlarm, Error};

#[derive(Deserialize, Serialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub(super) struct Alarm<G, Lpns>
where
    G: Group,
    Lpns: Group,
{
    below: PriceDTO<G, Lpns>,
    above: Option<PriceDTO<G, Lpns>>,
}

impl<G, Lpn, Lpns> TryFrom<Alarm<G, Lpns>> for ValidatedAlarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    type Error = Error;

    fn try_from(unchecked: Alarm<G, Lpns>) -> Result<Self, Self::Error> {
        unchecked.below.try_into().map_err(Into::into).and_then(|below| {
            unchecked
                .above
                .map(|above_dto| above_dto.try_into().map_err(Into::into))
                .transpose()
                .map(|above| Self { below, above })
                .and_then(|alarm| {
                    alarm.invariant_held().map(|()| alarm)
                })
        })
    }
}

impl<G, Lpn, Lpns> From<ValidatedAlarm<G, Lpn, Lpns>> for Alarm<G, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    fn from(validated: ValidatedAlarm<G, Lpn, Lpns>) -> Self {
        Self {
            below: validated.below.into(),
            above: validated.above.map(|base_price| base_price.into()),
        }
    }
}
