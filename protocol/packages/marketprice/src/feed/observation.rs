use serde::{Deserialize, Serialize};

use finance::price::Price;
use sdk::cosmwasm_std::{Addr, Timestamp};

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
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

#[track_caller]
pub fn valid_since<C, QuoteC>(since: Timestamp) -> impl Fn(&Observation<C, QuoteC>) -> bool
where
    C: 'static,
    QuoteC: 'static,
{
    move |o: &Observation<_, _>| o.time > since
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

    pub fn seen(&self, before_or_at: Timestamp) -> bool {
        self.time <= before_or_at
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
