use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-3/uausdc
            bank: "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11",
            // full ibc route: transfer/channel-3/uausdc
            dex: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-3/uausdc
            bank: "ibc/7DABB27AEEAFC0576967D342F21DC0944F5EA6584B45B9C635A3B3C35DCDA159",
            // full ibc route: transfer/channel-3/uausdc
            dex: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-208/uusdc
            bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
            // full ibc route: transfer/channel-208/uusdc
            dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
        },
    }
}
define_currency!(Usdc, USDC);
