use serde::{Deserialize, Serialize};

use finance::price::Price;
use sdk::cosmwasm_std::{Addr, Timestamp};

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Observation<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    feeder_addr: Addr,
    time: Timestamp,
    price: Price<C, QuoteC>,
}

impl<C, QuoteC> Observation<C, QuoteC> {
    pub fn new(
        feeder_addr: Addr,
        time: Timestamp,
        price: Price<C, QuoteC>,
    ) -> Observation<C, QuoteC> {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }

    pub fn feeder(&self) -> &Addr {
        &self.feeder_addr
    }

    pub fn price(&self) -> Price<C, QuoteC> {
        self.price
    }

    pub fn valid_since(&self, since: &Timestamp) -> bool {
        since < &self.time
    }
}

#[cfg(any(test, feature = "testing"))]
impl<C, QuoteC> Clone for Observation<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn clone(&self) -> Self {
        Self {
            feeder_addr: self.feeder_addr.clone(),
            time: self.time,
            price: self.price,
        }
    }
}
