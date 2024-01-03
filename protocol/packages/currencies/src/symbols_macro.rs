pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

macro_rules! define_symbol {
    (
        $currency: ident {
            $([$($net: literal),+ $(,)?]: { $($body:tt)* }),+ $(,)?
        } $(,)?
    ) => {
        pub const $currency: $crate::symbols_macro::CurrencySymbols = {
            $(
                #[cfg(any($(feature = $net),+))]
                { $crate::symbols_macro::CurrencySymbols { $($body)* } }
            )+
            #[cfg(all($($(not(feature = $net)),+),+))]
            compile_error!(concat!(stringify!($currency), " is not supported on the selected (if any) network! The currency is supported on the following networks: ", $($($net, ", "),+),+))
        };
    };
}

pub(crate) use define_symbol;
