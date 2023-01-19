use finance::currency::{Currency, Symbol};

pub trait CurrencyMapper<'a> {
    fn map<C>() -> Symbol<'a>
    where
        C: Currency;
}

pub mod local {
    use finance::currency::{Currency, Symbol};

    use super::CurrencyMapper;

    pub struct BankMapper {}
    impl<'a> CurrencyMapper<'a> for BankMapper {
        fn map<C>() -> Symbol<'a>
        where
            C: Currency,
        {
            C::BANK_SYMBOL
        }
    }
}

pub mod dex {
    use finance::currency::{AnyVisitor, AnyVisitorResult, Currency, Symbol, SymbolStatic};

    use crate::error::Error;

    use super::CurrencyMapper;

    pub struct DexMapper {}
    impl<'a> CurrencyMapper<'a> for DexMapper {
        fn map<C>() -> Symbol<'a>
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
            Ok(C::DEX_SYMBOL)
        }
    }
}
