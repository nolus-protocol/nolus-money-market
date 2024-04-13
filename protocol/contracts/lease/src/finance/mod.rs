#[cfg(feature = "contract")]
pub(crate) use contract::*;
#[cfg(feature = "contract")]
pub(crate) use currencies::Lpn as LpnCurrency;
pub(crate) use currencies::Lpns as LpnCurrencies;
use finance::coin::CoinDTO;

pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;

#[cfg(feature = "contract")]
pub type ReserveRef = reserve::stub::Ref<LpnCurrency>;

#[cfg(feature = "contract")]
mod contract;
