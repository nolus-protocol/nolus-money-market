use currency::CurrencyDef;
use error::BrokenInvariant;
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration, percent::Percent};
use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};

use crate::finance::LpnCurrency;

mod error;
mod unchecked;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(rename_all = "snake_case", try_from = "unchecked::NewConfig")]
pub struct NewConfig {
    pub lease_interest_rate_margin: Percent,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_due_period: Duration,
    pub lease_max_slippages: MaxSlippages,
}

impl NewConfig {
    fn invariant_held(&self) -> Result<(), BrokenInvariant<Self>> {
        let min_transaction: Coin<LpnCurrency> = self
            .lease_position_spec
            .min_transaction
            .as_specific(LpnCurrency::dto());
        BrokenInvariant::r#if(
            self.lease_max_slippages
                .liquidation
                .min_out(min_transaction)
                .is_zero(),
            "The min output from a dex transaction of the min transaction amount should be positive",
        )
    }
}
