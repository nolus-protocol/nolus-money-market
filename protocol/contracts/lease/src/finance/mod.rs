use finance::{coin::Coin, price::Price as GenericPrice};

use crate::api::LpnCurrency;

pub type LpnCoin = Coin<LpnCurrency>;
pub type Price<C> = GenericPrice<C, LpnCurrency>;
