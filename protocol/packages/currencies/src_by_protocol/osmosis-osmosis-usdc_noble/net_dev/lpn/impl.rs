use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_NOBLE {
        // full ibc route: transfer/channel-0/transfer/channel-???/uusdc
        bank: "ibc/NA_USDC_NOBLE",
        // full ibc route: transfer/channel-???/uusdc
        dex: "ibc/NA_USDC_NOBLE_DEX",
    }
}
define_currency!(UsdcNoble, USDC_NOBLE, 6);
