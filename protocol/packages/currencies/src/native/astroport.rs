use sdk::schemars;

use crate::{
    define_currency, define_symbol,
    ibc::macros::{local_native_on_dex_symbol, native_symbol},
};

define_symbol! {
    NLS {
        ["net_dev", "net_test", "net_main"]: {
            bank: native_symbol!("unls"),
            dex: local_native_on_dex_symbol!("unls"),
        },
    }
}
define_currency!(Nls, NLS);
