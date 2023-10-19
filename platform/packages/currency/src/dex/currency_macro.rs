pub use serde::{Deserialize, Serialize};

pub use sdk::schemars::{self, JsonSchema};

pub use crate::currency::{Currency, SymbolStatic};

#[macro_export]
macro_rules! define_currency {
    (
        $ident:ident,
        $ticker:path $(,)?
    ) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Default,
            $crate::dex::currency_macro::Serialize,
            $crate::dex::currency_macro::Deserialize,
            $crate::dex::currency_macro::JsonSchema,
        )]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub struct $ident {}

        impl $crate::dex::currency_macro::Currency for $ident {
            const TICKER: $crate::dex::currency_macro::SymbolStatic = ::core::stringify!($ticker);

            const BANK_SYMBOL: $crate::dex::currency_macro::SymbolStatic = $ticker.bank;

            const DEX_SYMBOL: $crate::dex::currency_macro::SymbolStatic = $ticker.dex;
        }
    };
}
