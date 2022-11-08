use finance::currency::{Currency, Symbol};

pub trait CurrencyMapper<'a> {
    fn map<C>() -> Symbol<'a>
    where
        C: Currency;
}

// A marker interface on #CurrencyMapper implementations to denote
// they should be used only on the local chain.
pub trait LocalChainCurrencyMapper {}

// A marker interface on #CurrencyMapper implementations to denote
// they should be used only on the DEX chain.
pub trait DexChainCurrencyMapper {}

pub mod local {
    use finance::currency::{Currency, Symbol};

    use super::{CurrencyMapper, LocalChainCurrencyMapper};

    pub struct BankMapper {}
    impl<'a> CurrencyMapper<'a> for BankMapper {
        fn map<C>() -> Symbol<'a>
        where
            C: Currency,
        {
            C::BANK_SYMBOL
        }
    }

    impl LocalChainCurrencyMapper for BankMapper {}
}

pub mod dex {
    use finance::currency::{AnyVisitor, Currency, Symbol, SymbolStatic};

    use crate::error::Error;

    use super::{CurrencyMapper, DexChainCurrencyMapper};

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

        fn on<C>(self) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
        {
            Ok(C::DEX_SYMBOL)
        }
    }

    impl DexChainCurrencyMapper for DexMapper {}
}
