use finance::{coin::Coin, price::Price as GenericPrice};
use lpp::stub::LppRef as LppGenericRef;

use super::{LpnCurrencies, LpnCurrency};

pub type LpnCoin = Coin<LpnCurrency>;
pub type Price<C> = GenericPrice<C, LpnCurrency>;

pub type LppRef = LppGenericRef<LpnCurrency, LpnCurrencies>;
