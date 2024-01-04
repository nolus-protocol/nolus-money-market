use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        ["net_dev"]: {
            bank: "unls",
            // full ibc route: transfer/channel-209/unls
            dex: "ibc/830F6CA3E33406586DFAADB25908769CB111046755EDAAD1D8D6A6E72A5E0C87"
        },
        ["net_test"]: {
            bank: "unls",
            // full ibc route: transfer/channel-208/unls
            dex: "ibc/C9F36A5FCF5FBD26661F9A09900301755C8B042696E4F456ACD73FAA7AFA6551"
        },
        ["net_main"]: {
            bank: "unls",
            // full ibc route: transfer/channel-44/unls
            dex: "ibc/6C9E6701AC217C0FC7D74B0F7A6265B9B4E3C3CDA6E80AADE5F950A8F52F9972"
        },
    }
}
define_currency!(Nls, NLS);
