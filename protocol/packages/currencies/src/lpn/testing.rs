use sdk::schemars;

use crate::{define_currency, define_symbol, Lpns};

define_symbol! {
    LPN {
        bank: "ibc/test_LPN",
        dex: "ibc/test_DEX_LPN",
    }
}
define_currency!(Lpn, LPN, Lpns, 6);
