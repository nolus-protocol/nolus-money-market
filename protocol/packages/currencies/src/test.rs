// Select some 5 distinct currencies
use crate::lease::{Atom as LC1, Axl as LC2, Cro as LC3, Osmo as LC4, Weth as LC5};
use crate::lpn::Lpn;
use crate::native::Nls;

pub type PaymentC1 = Nls;
pub type PaymentC2 = Lpn;
pub type PaymentC3 = LC1;
pub type PaymentC4 = LC2;
pub type PaymentC5 = LC3;
pub type PaymentC6 = LC4;
pub type PaymentC7 = LC5;

pub type LpnC = Lpn;

pub type LeaseC1 = LC1;
pub type LeaseC2 = LC2;
pub type LeaseC3 = LC3;
pub type LeaseC4 = LC4;
pub type LeaseC5 = LC5;

pub type NativeC = Nls;
