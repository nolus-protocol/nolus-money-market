use sdk::schemars;

use crate::{define_currency, define_symbol, Native};

define_symbol! {
    NLS {
        bank: "unls",
        // full ibc route: transfer/channel-8272/unls
        dex: "ibc/EF145240FE393A1CEC9C35ED1866A235D23176EA9B32069F714C9309FEA55718"
    }
}
define_currency!(Nls, NLS, Native, 6);
