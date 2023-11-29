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
        pub const $currency: $crate::symbols_macro::CurrencySymbols = {
            use $crate::symbols_macro::CurrencySymbols;

            $(
                #[cfg(any($(feature = $net),+))]
                { CurrencySymbols { $($body)* } }
            )+
            #[cfg(all($($(not(feature = $net)),+),+))]
            compile_error!(concat!("No symbols defined for \"", stringify!($currency), "\" selected network! Symbols defined for the following networks: ", $($($net, ", "),+),+))
        };
    };
}
