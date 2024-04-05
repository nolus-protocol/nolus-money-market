use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        ["net_dev"]: {
            bank: "unls",
            // full ibc route: transfer/channel-5733/unls
            dex: "ibc/48D5F90242DD5B460E139E1CCB503B0F7E44625CE7566BE74644F4600F5B5218"
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
