use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        ["dev"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-209/unls
            dex: "ibc/830F6CA3E33406586DFAADB25908769CB111046755EDAAD1D8D6A6E72A5E0C87"
        },
        ["test"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-208/unls
            dex: "ibc/C9F36A5FCF5FBD26661F9A09900301755C8B042696E4F456ACD73FAA7AFA6551"
        },
        ["main"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-?/unls
            dex: "ibc/NA_NLS_DEX"
        },
    }
}
define_currency!(Nls, NLS);
