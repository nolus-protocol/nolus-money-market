use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;

use crate::borrow::InterestRate;

use super::Config;

impl Config {
    pub(crate) fn new(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    ) -> Self {
        Self {
            lease_code,
            borrow_rate,
            min_utilization,
        }
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub const fn borrow_rate(&self) -> &InterestRate {
        &self.borrow_rate
    }

    pub const fn min_utilization(&self) -> BoundToHundredPercent {
        self.min_utilization
    }

    pub fn initial_derivative_price<Lpn>() -> Price<NLpn, Lpn>
    where
        Lpn: 'static,
    {
        // must be >= 1 to guarantee proper (non zero result) Nlpn -> Lpn conversion
        Price::identity()
    }
}
