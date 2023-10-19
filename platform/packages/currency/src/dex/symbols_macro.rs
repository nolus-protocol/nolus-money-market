pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

#[macro_export]
macro_rules! define_symbol {
    (
        $currency: ident {
            $([$($net: literal),+ $(,)?]: { $($body:tt)* }),+ $(,)?
        } $(,)?
    ) => {
        pub const $currency: $crate::dex::symbols_macro::CurrencySymbols = {
            use $crate::dex::symbols_macro::CurrencySymbols;

            $(
                #[cfg(any($(net = $net),+))]
                { CurrencySymbols { $($body)* } }
            )+
            #[cfg(all($($(not(net = $net)),+),+))]
            { compile_error!(concat!("No symbols defined for network with name \"", env!("NET"), "\"!")) }
        };
    };
}
