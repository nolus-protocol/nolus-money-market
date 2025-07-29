use serde::{Deserialize, Serialize};

use finance::percent::Percent100;
use platform::contract::Code;

use crate::borrow::InterestRate;

#[cfg(feature = "contract")]
mod r#impl;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: Percent100,
}
