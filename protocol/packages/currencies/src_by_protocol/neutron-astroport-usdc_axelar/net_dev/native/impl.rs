use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/0D9EB2C9961610CD2F04003188C0B713E72297DCBC32371602897069DC0E3055", // transfer/channel-585/unls
    Native,
    6
);
