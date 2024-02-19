use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        ["net_dev"]: {
            bank: "unls",
            // full ibc route: transfer/channel-571/unls
            dex: "ibc/D1FAFE8009558038F94B9478D5066D633614DCD4CD78D4977BBC855DEDD36C91"
        },
        ["net_test"]: {
            bank: "unls",
            // full ibc route: transfer/channel-5598/unls
            dex: "ibc/633DEE69AD15A09EFD664F665491018111B60FE1CE3A8286ECF4BECFED59A5CB"
        },
        ["net_main"]: {
            bank: "unls",
            // full ibc route: transfer/channel-44/unls
            dex: "ibc/6C9E6701AC217C0FC7D74B0F7A6265B9B4E3C3CDA6E80AADE5F950A8F52F9972"
        },
    }
}
define_currency!(Nls, NLS);
