use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/test_DEX_NLS",
    Native,
    6,
);
