use crate::{define_currency, define_symbol};

define_symbol! {
    NLS {
        bank: "unls",
        dex: "ibc/DEADCODEDEADCODE"
    }
}
define_currency!(Nls, NLS);
