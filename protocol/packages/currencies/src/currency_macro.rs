pub use serde::{Deserialize, Serialize};

pub use sdk::schemars::{self, JsonSchema};

pub use currency::{CurrencyDTO, CurrencyDef, Definition};

#[macro_export]
macro_rules! define_currency {
    (
        $ident:ident,
        $bank_symbol: literal,
        $dex_symbol: literal,
        $group:ty,
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

        impl $crate::currency_macro::CurrencyDef for $ident {
            type Group = $group;

            fn definition() -> &'static Self {
                const INSTANCE: &$ident = &$ident($crate::currency_macro::CurrencyDTO::new(
                    &$crate::currency_macro::Definition::new(
                        ::core::stringify!($ticker),
                        $bank_symbol,
                        $dex_symbol,
                        $decimal_digits,
                    ),
                ));

                INSTANCE
            }

            fn dto(&self) -> &$crate::currency_macro::CurrencyDTO<Self::Group> {
                &self.0
            }
        }
    };
}

define_currency! {
    Nls,
    "unls",
    "ibc/unls",
    crate::lease::LeaseGroup,
    6,
}
