use serde::{Deserialize, Serialize};

use finance::currency::{Currency, SymbolStatic};

#[macro_export]
macro_rules! define_currency {
    (
        $ident:ident,
        $ticker:path $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, $crate::define::Serialize, $crate::define::Deserialize)]
        pub struct $ident {}

        impl $crate::currency_macro::Currency for $ident {
            const TICKER: SymbolStatic = ::core::stringify!($ticker);

            const BANK_SYMBOL: SymbolStatic = $ticker.bank;

            const DEX_SYMBOL: SymbolStatic = $ticker.dex;
        }
    };
}
