use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/D9AFCECDD361D38302AA66EB3BAC23B95234832C51D12489DC451FA2B7C72782", // transfer/channel-783/unls
    Native,
    6
);
