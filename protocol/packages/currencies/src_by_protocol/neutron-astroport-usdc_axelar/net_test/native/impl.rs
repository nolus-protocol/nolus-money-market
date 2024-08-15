use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "unls",
    "ibc/E808FAAE7ADDA37453A8F0F67D74669F6580CBA5EF0F7889D46FB02D282098E3", // transfer/channel-1061/unls
    Native,
    6
);
