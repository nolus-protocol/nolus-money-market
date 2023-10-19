// #[cfg(any(test, dex = "osmosis"))]
#[cfg(dex = "osmosis")]
mod currencies {
    use crate::dex::{
        lease::osmosis::{Atom, Axl, Cro, Osmo, Weth},
        lpn::osmosis::Usdc,
        native::osmosis::Nls,
    };

    pub type PaymentC1 = Nls;
    pub type PaymentC2 = Usdc;
    pub type PaymentC3 = Atom;
    pub type PaymentC4 = Axl;
    pub type PaymentC5 = Osmo;
    pub type PaymentC6 = Cro;
    pub type PaymentC7 = Weth;

    pub type StableC1 = Usdc;

    pub type NativeC = Nls;
}
pub use currencies::*;
