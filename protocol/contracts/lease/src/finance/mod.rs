#[cfg(feature = "contract")]
pub(crate) use contract::*;
pub(crate) use currencies::Lpns as LpnCurrencies;
use finance::coin::CoinDTO;

pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;

pub type LppRef = lpp::stub::LppRef<LpnCurrency, LpnCurrencies>;

pub type ReserveRef = reserve::stub::Ref<LpnCurrency>;

#[cfg(feature = "contract")]
mod contract;
