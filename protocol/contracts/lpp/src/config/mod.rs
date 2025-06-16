use serde::{Deserialize, Serialize};

use finance::percent::bound::BoundToHundredPercent;
use platform::contract::Code;
use sdk::cosmwasm_std::Addr;

use crate::borrow::InterestRate;

#[cfg(feature = "contract")]
mod r#impl;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
    lease_code_admin: Addr,
}
