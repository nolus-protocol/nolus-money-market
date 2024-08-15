use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(Nls, "unls", "ibc/test_DEX_NLS", Native, 6);
