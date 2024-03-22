use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_AXELAR {
        ["net_dev"]: {
            // full ibc route: transfer/channel-1/transfer/channel-8/uausdc
            bank: "ibc/88012ABE034CE754022417BFEDF29F8B16C5B3338386EA20298ADCECA8329019",
            // full ibc route: transfer/channel-8/uausdc
            dex: "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1990/transfer/channel-8/uausdc
            bank: "ibc/88E889952D6F30CEFCE1B1EE4089DA54939DE44B0A7F11558C230209AF228937",
            // full ibc route: transfer/channel-8/uausdc
            dex: "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-3839/transfer/channel-2/uusdc
            bank: "ibc/076CF690A9912E0B7A2CCA75B719D68AF7C20E4B0B6460569B333DDEB19BBBA1",
            // full ibc route: transfer/channel-2/uusdc
            dex: "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349",
        },
    }
}
define_currency!(UsdcAxelar, USDC_AXELAR, 6);
