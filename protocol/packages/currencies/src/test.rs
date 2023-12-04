use crate::native::Nls;
#[cfg(feature = "astroport")]
use crate::{
    lease::astroport::{Atom as LC1, Dydx as LC4, Ntrn as LC2, StAtom as LC3, TestC1 as LC5},
    lpn::astroport::UsdcAxelar as SC1,
};
#[cfg(feature = "osmosis")]
use crate::{
    lease::osmosis::{Atom as LC1, Axl as LC2, Cro as LC3, Osmo as LC4, Weth as LC5},
    lpn::osmosis::Usdc as SC1,
};

pub type PaymentC1 = Nls;
pub type PaymentC2 = SC1;
pub type PaymentC3 = LC1;
pub type PaymentC4 = LC2;
pub type PaymentC5 = LC3;
pub type PaymentC6 = LC4;
pub type PaymentC7 = LC5;

pub type StableC1 = SC1;

pub type LeaseC1 = LC1;
pub type LeaseC2 = LC2;
pub type LeaseC3 = LC3;
pub type LeaseC4 = LC4;
pub type LeaseC5 = LC5;

pub type NativeC = Nls;
