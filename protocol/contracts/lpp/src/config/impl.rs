use finance::{percent::Percent100, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;
use sdk::cosmwasm_std::Addr;

use crate::borrow::InterestRate;

use super::Config;

impl Config {
    pub(crate) fn new(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: Percent100,
        protocol_admin: Addr,
    ) -> Self {
        Self {
            lease_code,
            borrow_rate,
            min_utilization,
            protocol_admin,
        }
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub const fn borrow_rate(&self) -> &InterestRate {
        &self.borrow_rate
    }

    pub const fn min_utilization(&self) -> Percent100 {
        self.min_utilization
    }

    pub const fn protocol_admin(&self) -> &Addr {
        &self.protocol_admin
    }

    pub fn initial_derivative_price<Lpn>() -> Price<NLpn, Lpn>
    where
        Lpn: 'static,
    {
        // must be >= 1 to guarantee proper (non zero result) Nlpn -> Lpn conversion
        Price::identity()
    }
}
