pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

#[macro_export]
macro_rules! define_symbol {
    (
        $currency: ident { $($body:tt)* } $(,)?
    ) => {
        const $currency_DEFINITION: Definition = Definition::new(
            ::core::stringify!($ticker),
            $ticker.bank,
            $ticker.dex,
            $decimal_digits,
        );

        const $currency: $crate::symbols_macro::CurrencySymbols = {
            $crate::symbols_macro::CurrencySymbols { $($body)* }
        };
        const $currency: NlsPlatform = NlsPlatform(CurrencyDTO::new(&NLS_PLATFORM_DEFINITION));
    };
}
