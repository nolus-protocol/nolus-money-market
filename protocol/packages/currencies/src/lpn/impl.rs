use sdk::schemars;

use crate::{define_currency, define_symbol, Lpns};

define_symbol! {
    USDC {
        // full ibc route: transfer/channel-0/transfer/channel-208/uusdc
        bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
        // full ibc route: transfer/channel-208/uusdc
        dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
    }
}
define_currency!(Usdc, USDC, Lpns, 6);

pub use Usdc as Lpn;
