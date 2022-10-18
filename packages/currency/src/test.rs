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
    const BANK_SYMBOL: SymbolStatic = "ibc/TestCurrencyA";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyB;

impl Currency for TestCurrencyB {
    const TICKER: SymbolStatic = "TestCurrencyB";
    const BANK_SYMBOL: SymbolStatic = "ibc/TestCurrencyB";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyC;

impl Currency for TestCurrencyC {
    const TICKER: SymbolStatic = "TestCurrencyC";
    const BANK_SYMBOL: SymbolStatic = "ibc/TestCurrencyC";
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]

pub struct TestCurrencyD;

impl Currency for TestCurrencyD {
    const TICKER: SymbolStatic = "TestCurrencyD";
    const BANK_SYMBOL: SymbolStatic = "ibc/TestCurrencyD";
}

impl Member<PaymentGroup> for TestCurrencyA {}
impl Member<PaymentGroup> for TestCurrencyB {}
impl Member<PaymentGroup> for TestCurrencyC {}
impl Member<PaymentGroup> for TestCurrencyD {}
