use finance::price::Price as GenericPrice;

use crate::api::LpnCurrency;

pub type Price<C> = GenericPrice<C, LpnCurrency>;
