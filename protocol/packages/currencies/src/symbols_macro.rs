pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

#[macro_export]
macro_rules! define_symbol {
    (
        $currency: ident { $($body:tt)* } $(,)?
    ) => {
        pub const $currency: $crate::symbols_macro::CurrencySymbols = {
            $crate::symbols_macro::CurrencySymbols { $($body)* }
        };
    };
}
