use sdk::schemars;

use crate::{define_currency, Native};

define_currency!(
    Nls,
    "unls",
    "ibc/48D5F90242DD5B460E139E1CCB503B0F7E44625CE7566BE74644F4600F5B5218", // transfer/channel-5733/unls
    Native,
    6
);
