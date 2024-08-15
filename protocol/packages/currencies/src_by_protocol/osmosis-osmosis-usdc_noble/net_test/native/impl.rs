use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "unls",
    "ibc/EF145240FE393A1CEC9C35ED1866A235D23176EA9B32069F714C9309FEA55718", // transfer/channel-8272/unls
    Native,
    6
);
