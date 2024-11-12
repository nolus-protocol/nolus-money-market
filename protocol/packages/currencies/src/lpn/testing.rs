use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef, Definition, InPoolWith};
use sdk::schemars::JsonSchema;

use crate::{
    lease::{LeaseC2, LeaseC7},
    native::Nls,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct Lpn(CurrencyDTO<super::Group>);

impl CurrencyDef for Lpn {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Lpn(CurrencyDTO::new(
                const { &Definition::new("LPN", "ibc/bank_LPN", "ibc/dex_LPN", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl InPoolWith<LeaseC2> for Lpn {}

impl InPoolWith<LeaseC7> for Lpn {}

impl InPoolWith<Nls> for Lpn {}
