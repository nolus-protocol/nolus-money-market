use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        bank: "unls",
        // full ibc route: transfer/channel-783/unls
        dex: "ibc/D9AFCECDD361D38302AA66EB3BAC23B95234832C51D12489DC451FA2B7C72782"
    }
}
define_currency!(Nls, NLS, 6);
