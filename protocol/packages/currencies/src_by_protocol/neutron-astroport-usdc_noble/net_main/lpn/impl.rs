use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_NOBLE {
        // full ibc route: transfer/channel-3839/transfer/channel-30/uusdc
        bank: "ibc/18161D8EFBD00FF5B7683EF8E923B8913453567FBE3FB6672D75712B0DEB6682",
        // full ibc route: transfer/channel-30/uusdc
        dex: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81",
    }
}
define_currency!(UsdcNoble, USDC_NOBLE, 6);

pub use UsdcNoble as Lpn;
