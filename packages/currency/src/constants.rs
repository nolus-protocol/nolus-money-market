pub struct CurrencySymbols {
    pub bank: &'static str,
    pub dex: &'static str,
}

macro_rules! with_fallback {
    (() / ($($fallback:tt)*)) => { $($fallback)* };
    (($($primary:tt)+) / ($($fallback:tt)*)) => { $($primary)+ };
}

macro_rules! def {
    (
        $(
            $currency: ident {
                $(
                    $net:literal => { $($body:tt)* }
                )*
                $(
                    _ => { $($default_body:tt)* }
                )?
            }
        ),* $(,)?
    ) => {
        $(
            $(
                #[cfg(net_name = $net)]
                pub const $currency: CurrencySymbols = CurrencySymbols { $($body)* };
            )*

            #[cfg(not(any($(net_name = $net), *)))]
            pub const $currency: CurrencySymbols = with_fallback!(
                (
                    $(
                        CurrencySymbols { $($default_body)* }
                    )?
                ) / (
                    ::core::compile_error!(::core::concat!("No fallback arguments provided for currency \"", ::core::stringify!($currency), "\"!"))
                )
            );
        )*
    };
}

#[cfg(test)]
def! {
    TestingCurrency {
        "main_net" => {
            bank: "1",
            dex: "2",
        }
        "test_net" => {
            bank: "3",
            dex: "4",
        }
        _ => {
            bank: "5",
            dex: "6",
        }
    }
}
