use finance::liability::Liability;
use serde::{Deserialize, Serialize};

use crate::api::{LeaseCoin, LpnCoin};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct PositionDTO {
    pub amount: LeaseCoin,
    pub liability: Liability,
    min_asset: LpnCoin,
    min_sell_asset: LpnCoin,
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
            liability,
            min_asset,
            min_sell_asset,
        }
    }
}
