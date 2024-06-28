pub use currency::{Currency, SymbolStatic, Symbols};

#[macro_export]
macro_rules! define_currency {
    (
        $ident:ident,
        $ticker:path,
        $group:ident,
        $decimal_digits:literal $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
        pub struct $ident {}

        impl $crate::currency_macro::Currency for $ident {
            type Group = $group;
        }

        impl $crate::currency_macro::Symbols for $ident {
            const TICKER: $crate::currency_macro::SymbolStatic = ::core::stringify!($ticker);

            const BANK_SYMBOL: $crate::currency_macro::SymbolStatic = $ticker.bank;

            const DEX_SYMBOL: $crate::currency_macro::SymbolStatic = $ticker.dex;

            const DECIMAL_DIGITS: u8 = $decimal_digits;
        }
    };
}
