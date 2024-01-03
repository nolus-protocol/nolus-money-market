macro_rules! to_const_str {
    ($ibc_symbol: expr $(,)?) => {{
        const SYMBOL: $crate::ibc::IbcSymbolArray = $ibc_symbol;

        if let Ok(symbol) = std::str::from_utf8(&SYMBOL) {
            symbol
        } else {
            panic!("Provided IBC symbol is not a valid UTF-8 encoded string!")
        }
    }};
}

pub(crate) use to_const_str;

macro_rules! native_symbol {
    ($symbol: literal $(,)?) => {
        $crate::symbols_macro::BankSymbol($symbol)
    };
}

pub(crate) use native_symbol;

macro_rules! bank_symbol {
    ([$($($channel: literal),+ $(,)?)?], $symbol: literal $(,)?) => {
        $crate::symbols_macro::BankSymbol($crate::ibc::macros::to_const_str!($crate::ibc::bank_symbol(&[$($($channel),+)?], $symbol)))
    };
}

pub(crate) use bank_symbol;

macro_rules! local_native_on_dex_symbol {
    ($symbol: literal $(,)?) => {
        $crate::symbols_macro::DexSymbol($crate::ibc::macros::to_const_str!(
            $crate::ibc::local_native_on_dex_symbol($symbol)
        ))
    };
}

pub(crate) use local_native_on_dex_symbol;

macro_rules! dex_symbol {
    ([$($channel: literal),+ $(,)?], $symbol: literal $(,)?) => {
        $crate::symbols_macro::DexSymbol($crate::ibc::macros::to_const_str!($crate::ibc::dex_symbol(&[$($channel),+], $symbol)))
    };
}

pub(crate) use dex_symbol;

macro_rules! dex_native_symbol {
    ($symbol: literal $(,)?) => {
        $crate::symbols_macro::DexSymbol($symbol)
    };
}

pub(crate) use dex_native_symbol;
