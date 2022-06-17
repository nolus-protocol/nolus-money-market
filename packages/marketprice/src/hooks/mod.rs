use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod errors;
pub mod msg;
pub mod price_hooks;

pub trait Rule {
    fn should_run(&self) -> bool;
    fn evaluate(&self);
}

pub trait PriceRule: Rule {}

#[derive(Clone, Debug, PartialEq, JsonSchema, Deserialize, Serialize)]
pub struct SimpleRule {}

impl Rule for SimpleRule {
    fn should_run(&self) -> bool {
        true
    }

    fn evaluate(&self) {
        println!("SimpleRule is executed!")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceBelowRule {}

impl Rule for PriceBelowRule {
    fn should_run(&self) -> bool {
        todo!()
    }

    fn evaluate(&self) {
        todo!()
    }
}

impl PriceRule for PriceBelowRule {}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceAboveRule {}

impl Rule for PriceAboveRule {
    fn should_run(&self) -> bool {
        todo!()
    }

    fn evaluate(&self) {
        todo!()
    }
}

impl PriceRule for PriceAboveRule {}
