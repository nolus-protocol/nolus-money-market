use finance::{currency::Currency, duration::Duration, price::Price};
use sdk::cosmwasm_std::{Addr, Timestamp};
use serde::{Deserialize, Serialize};

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
pub fn valid_at<C, QuoteC>(
    at: Timestamp,
    period: Duration,
) -> impl FnMut(&Observation<C, QuoteC>) -> bool
where
    C: Currency,
    QuoteC: Currency,
{
    move |o: &Observation<_, _>| {
        debug_assert!(o.seen(at));
        Duration::between(o.time, at) < period
    }
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
