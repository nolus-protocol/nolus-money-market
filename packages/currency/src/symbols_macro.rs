pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

#[macro_export]
macro_rules! define_symbol {
    (
        $currency: ident {
            { $($default_body:tt)* },
            alt: { $($gated_body:tt)* } $(,)?
        } $(,)?
    ) => {
        pub const $currency: $crate::symbols_macro::CurrencySymbols = {
            use $crate::symbols_macro::CurrencySymbols;

            #[cfg(not(feature = "alt_net_symbols"))]
            { CurrencySymbols { $($default_body)* } }
            #[cfg(feature = "alt_net_symbols")]
            { CurrencySymbols { $($gated_body)* } }
        };
    };
}
