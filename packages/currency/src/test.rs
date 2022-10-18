use serde::{Deserialize, Serialize};

use finance::currency::{Currency, Member, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

use crate::payment::PaymentGroup;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyA;

impl Currency for TestCurrencyA {
    const TICKER: SymbolStatic = "TestCurrencyA";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyB;

impl Currency for TestCurrencyB {
    const TICKER: SymbolStatic = "TestCurrencyB";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyC;

impl Currency for TestCurrencyC {
    const TICKER: SymbolStatic = "TestCurrencyC";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyD;

impl Currency for TestCurrencyD {
    const TICKER: SymbolStatic = "TestCurrencyD";
}

impl Member<PaymentGroup> for TestCurrencyA {}
impl Member<PaymentGroup> for TestCurrencyB {}
impl Member<PaymentGroup> for TestCurrencyC {}
impl Member<PaymentGroup> for TestCurrencyD {}
