use finance::currency::{Currency, Member, SymbolStatic};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::payment::PaymentGroup;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyA;

impl Currency for TestCurrencyA {
    const SYMBOL: SymbolStatic = "TestCurrencyA";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyB;

impl Currency for TestCurrencyB {
    const SYMBOL: SymbolStatic = "TestCurrencyB";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyC;

impl Currency for TestCurrencyC {
    const SYMBOL: SymbolStatic = "TestCurrencyC";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyD;

impl Currency for TestCurrencyD {
    const SYMBOL: SymbolStatic = "TestCurrencyD";
}

impl Member<PaymentGroup> for TestCurrencyA {}
impl Member<PaymentGroup> for TestCurrencyB {}
impl Member<PaymentGroup> for TestCurrencyC {}
impl Member<PaymentGroup> for TestCurrencyD {}
