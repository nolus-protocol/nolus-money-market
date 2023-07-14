use currency::{
    lease::{Atom, Cro},
    lpn::Usdc,
};
use finance::coin::Coin;

mod close;
mod compare_with_lpp;
mod helpers;
mod liquidation;
mod open;
mod repay;

type Lpn = Usdc;
type LpnCoin = Coin<Lpn>;

type LeaseCurrency = Cro;
type LeaseCoin = Coin<LeaseCurrency>;

type PaymentCurrency = Atom;
type PaymentCoin = Coin<PaymentCurrency>;

const DOWNPAYMENT: u128 = 1_000_000_000_000;
