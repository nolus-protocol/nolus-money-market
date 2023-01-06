use serde::{Deserialize, Serialize};

use currency::payment::PaymentGroup;
use finance::currency::SymbolOwned;

pub mod error;
#[cfg(feature = "trx")]
pub mod trx;

pub type PoolId = u64;
pub type SwapGroup = PaymentGroup;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SwapTarget {
    pub pool_id: PoolId,
    pub target: SymbolOwned,
}

pub type SwapPath = Vec<SwapTarget>;
