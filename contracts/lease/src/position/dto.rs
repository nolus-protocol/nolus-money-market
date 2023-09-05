use finance::liability::Liability;
use serde::{Deserialize, Serialize};

use crate::api::{LeaseCoin, LpnCoin, PositionSpec};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct PositionDTO {
    pub amount: LeaseCoin,
    pub spec: PositionSpec,
}

impl PositionDTO {
    pub(crate) fn new(
        amount: LeaseCoin,
        liability: Liability,
        min_asset: LpnCoin,
        min_sell_asset: LpnCoin,
    ) -> Self {
        Self {
            amount,
            spec: PositionSpec::new(liability, min_asset, min_sell_asset),
        }
    }
}
