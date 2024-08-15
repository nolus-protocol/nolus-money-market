use sdk::schemars;

use crate::{define_currency, Lpns};

define_currency!(Lpn, "ibc/test_LPN", "ibc/test_DEX_LPN", Lpns, 6);
