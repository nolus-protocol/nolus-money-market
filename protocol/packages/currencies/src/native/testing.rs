use sdk::schemars;

use crate::{define_currency, define_symbol, Native};

define_symbol! {
    NLS {
        bank: "unls",
        dex: "ibc/test_DEX_NLS",
    }
}
define_currency!(Nls, NLS, Native, 6);
