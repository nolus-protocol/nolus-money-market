pub use serde::{Deserialize, Serialize};

pub use sdk::schemars::JsonSchema;

pub use currency::{Currency, Definition, SymbolStatic};

#[macro_export]
macro_rules! define_currency {
    (
        $ident:ident,
        $ticker:path,
        $group:ident,
        $decimal_digits:literal $(,)?
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
            $crate::currency_macro::Serialize,
            $crate::currency_macro::Deserialize,
            $crate::currency_macro::JsonSchema,
        )]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub struct $ident {}

        impl $crate::currency_macro::Currency for $ident {
            type Group = $group;
        }

        impl $crate::currency_macro::Definition for $ident {
            const TICKER: $crate::currency_macro::SymbolStatic = ::core::stringify!($ticker);

            const BANK_SYMBOL: $crate::currency_macro::SymbolStatic = $ticker.bank;

            const DEX_SYMBOL: $crate::currency_macro::SymbolStatic = $ticker.dex;

            const DECIMAL_DIGITS: u8 = $decimal_digits;
        }
    };
}
