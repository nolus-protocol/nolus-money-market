use std::fmt::Debug;

use crate::price::Price;

#[derive(Debug)]
struct AveragePrice<C, QouteC> {
    // An instance of AveragePrice will have at least one Price
    total: Price<C, QouteC>,
    // > 0; TODO: replace with Count
    count: u32,
}

impl<C, QuoteC> AveragePrice<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    fn new(initial: Price<C, QuoteC>) -> Self {
        Self {
            total: initial,
            count: 1,
        }
    }
}
