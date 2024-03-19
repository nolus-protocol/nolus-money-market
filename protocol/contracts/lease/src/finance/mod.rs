#[cfg(feature = "contract")]
pub(crate) use contract::*;
pub(crate) use currencies::Lpns as LpnCurrencies;
use finance::coin::CoinDTO;

pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;

#[cfg(feature = "contract")]
mod contract;
