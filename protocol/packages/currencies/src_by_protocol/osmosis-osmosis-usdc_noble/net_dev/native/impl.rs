use sdk::schemars;

use crate::{define_currency, define_symbol, Native};

define_symbol! {
    NLS {
        bank: "unls",
        // full ibc route: transfer/channel-5733/unls
        dex: "ibc/48D5F90242DD5B460E139E1CCB503B0F7E44625CE7566BE74644F4600F5B5218"
    }
}
define_currency!(Nls, NLS, Native, 6);
