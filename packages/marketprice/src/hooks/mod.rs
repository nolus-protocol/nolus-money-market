use crate::feed::Price;

pub mod errors;
pub mod price;

pub trait Rule {
    fn should_run(&self, current_price: Price) -> bool;
    fn evaluate(&self);
}

pub trait PriceRule: Rule {}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceBelowRule {}

impl Rule for PriceBelowRule {
    fn should_run(&self, _current_price: Price) -> bool {
        todo!()
    }

    fn evaluate(&self) {
        todo!()
    }
}
