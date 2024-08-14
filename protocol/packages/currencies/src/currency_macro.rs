pub use serde::{Deserialize, Serialize};

pub use sdk::schemars::JsonSchema;

pub use currency::{CurrencyDTO, CurrencyDef, Definition, SymbolStatic};

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
            $crate::currency_macro::Serialize,
            $crate::currency_macro::Deserialize,
            $crate::currency_macro::JsonSchema,
        )]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub struct $ident($crate::currency_macro::CurrencyDTO<$group>);

        const $ticker_DEFINITION: Definition = Definition::new(
            ::core::stringify!($ticker),
            $ticker.bank,
            $ticker.dex,
            $decimal_digits,
        );

        const $ticker: $ident = $ident($crate::currency_macro::CurrencyDTO::new(&$ticker_DEFINITION))

        impl $crate::currency_macro::CurrencyDef for $ident {
            type Group = $group;

            fn definition() -> &'static Self {
                &$ticker
            }

            fn dto(&self) -> &CurrencyDTO<Self::Group> {
                &self.0
            }
        }

    };
}
