pub use currencies::Lpn as LpnCurrency;
use finance::{coin::Coin, price::Price as GenericPrice};
use lpp::stub::LppRef as LppGenericRef;
use oracle_platform::OracleRef as OracleGenericRef;

use super::LpnCurrencies;

pub type LpnCoin = Coin<LpnCurrency>;
pub type Price<C> = GenericPrice<C, LpnCurrency>;

pub type LppRef = LppGenericRef<LpnCurrency>;
pub type OracleRef = OracleGenericRef<LpnCurrency, LpnCurrencies>;
pub type ReserveRef = reserve::stub::Ref<LpnCurrency>;
