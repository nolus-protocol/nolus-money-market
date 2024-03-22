use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        ["net_dev"]: {
            bank: "unls",
            // full ibc route: transfer/channel-109/unls
            dex: "ibc/5E7589614F0B4B80D91923D15D8EB0972AAA6226F7566921F1D6A07EA0DB0D2C"
        },
        ["net_test"]: {
            bank: "unls",
            // full ibc route: transfer/channel-4508/unls
            dex: "ibc/1588A50E9EF2B6E45B443B8AF5AD7891996D7104566908982603D73D1956FE51"
        },
        ["net_main"]: {
            bank: "unls",
            // full ibc route: transfer/channel-783/unls
            dex: "ibc/D9AFCECDD361D38302AA66EB3BAC23B95234832C51D12489DC451FA2B7C72782"
        },
    }
}
define_currency!(Nls, NLS, 6);
