use crate::{currency_macro::schemars, define_currency, define_symbol};

define_symbol! {
    NLS {
        {
            bank: "unls",
            dex: "ibc/DEADCODEDEADCODE"
        },
        alt: {
            bank: "unls",
            dex: "ibc/DEADCODEDEADCODE"
        },
    }
}
define_currency!(Nls, NLS);
