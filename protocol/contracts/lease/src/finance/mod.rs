#[cfg(feature = "contract")]
pub(crate) use contract::*;
pub(crate) use currencies::{Lpn as LpnCurrency, Lpns as LpnCurrencies};
use finance::coin::CoinDTO;

pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;

pub type ReserveRef = reserve::stub::Ref<LpnCurrency>;

#[cfg(feature = "contract")]
mod contract;
