use serde::{Deserialize, Serialize};

use currencies::Lpns;
use currency::{CurrencyDef, MemberOf};
use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;

use crate::{borrow::InterestRate, msg::InstantiateMsg};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
}

impl Config {
    pub(crate) fn new<Lpn>(msg: InstantiateMsg, lease_code: Code) -> Self
    where
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<Lpns>,
    {
        debug_assert_eq!(Ok(()), msg.lpn.of_currency(Lpn::dto()));
        Self {
            lease_code,
            borrow_rate: msg.borrow_rate,
            min_utilization: msg.min_utilization,
        }
    }

    pub(crate) fn new_unchecked(
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
        Price::identity()
    }
}
