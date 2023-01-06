use serde::{Deserialize, Serialize};

use finance::{currency::Currency, price::Price};
use sdk::cosmwasm_std::{Addr, Timestamp};

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub struct Observation<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    feeder_addr: Addr,
    time: Timestamp,
    price: Price<C, QuoteC>,
}

#[track_caller]
pub fn valid_since<C, QuoteC>(since: Timestamp) -> impl FnMut(&Observation<C, QuoteC>) -> bool
where
    C: Currency,
    QuoteC: Currency,
{
    move |o: &Observation<_, _>| o.time > since
}

impl<C, QuoteC> Observation<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
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
