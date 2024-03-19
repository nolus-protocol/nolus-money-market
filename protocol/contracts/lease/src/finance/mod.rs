pub(crate) use currencies::{Lpn as LpnCurrency, Lpns as LpnCurrencies};
use finance::{
    coin::{Coin, CoinDTO},
    price::Price as GenericPrice,
};
use lpp::stub::LppRef as LppGenericRef;

pub type LpnCoin = Coin<LpnCurrency>;
pub type Price<C> = GenericPrice<C, LpnCurrency>;

pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;

pub type LppRef = LppGenericRef<LpnCurrency, LpnCurrencies>;
