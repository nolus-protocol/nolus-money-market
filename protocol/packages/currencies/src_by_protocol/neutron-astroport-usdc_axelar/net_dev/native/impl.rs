use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/40A9BC802B6F2B51B3B9A6D2615EB8A9666755E987CABE978980CD6F08F31E1D", // transfer/channel-1035/unls
    Native,
    6
);
