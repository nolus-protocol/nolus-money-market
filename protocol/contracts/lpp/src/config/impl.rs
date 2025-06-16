use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;
use sdk::cosmwasm_std::Addr;

use crate::borrow::InterestRate;

use super::Config;

impl Config {
    pub(crate) fn new(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
        lease_code_admin: Addr,
    ) -> Self {
        Self {
            lease_code,
            borrow_rate,
            min_utilization,
            lease_code_admin,
        }
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub const fn lease_code_admin(&self) -> Addr {
        self.lease_code_admin
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
        Price::identity()
    }
}
