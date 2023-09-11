use currency::{Currency, SymbolSlice};

pub trait CurrencyMapper<'a> {
    fn map<C>() -> &'a SymbolSlice
    where
        C: Currency;
}

pub mod local {
    use currency::{Currency, SymbolSlice};

    use super::CurrencyMapper;

    pub struct BankMapper {}
    impl<'a> CurrencyMapper<'a> for BankMapper {
        fn map<C>() -> &'a SymbolSlice
        where
            C: Currency,
        {
            C::BANK_SYMBOL
        }
    }
}

pub mod dex {
    use currency::{AnyVisitor, AnyVisitorResult, Currency, SymbolSlice, SymbolStatic};

    use crate::error::Error;

    use super::CurrencyMapper;

    pub struct DexMapper {}
    impl<'a> CurrencyMapper<'a> for DexMapper {
        fn map<C>() -> &'a SymbolSlice
        where
            C: Currency,
        {
            C::DEX_SYMBOL
        }
    }

    impl AnyVisitor for DexMapper {
        type Output = SymbolStatic;
        type Error = Error;

        fn on<C>(self) -> AnyVisitorResult<Self>
        where
            C: Currency,
        {
            Ok(DexMapper::map::<C>())
        }
    }
}
