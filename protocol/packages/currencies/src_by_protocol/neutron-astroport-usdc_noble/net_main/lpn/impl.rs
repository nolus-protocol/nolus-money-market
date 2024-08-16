use sdk::schemars;

use crate::{define_currency, Lpns};

define_currency!(
    UsdcNoble,
    "USDC_NOBLE",
    "ibc/18161D8EFBD00FF5B7683EF8E923B8913453567FBE3FB6672D75712B0DEB6682", // transfer/channel-3839/transfer/channel-30/uusdc
    "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81", // transfer/channel-30/uusdc
    Lpns,
    6
);

pub use UsdcNoble as Lpn;
