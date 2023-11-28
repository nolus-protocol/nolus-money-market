#[cfg(not(any(feature = "astroport", feature = "osmosis")))]
compile_error!("No dex selected!");

#[cfg(not(any(net = "dev", net = "test", net = "main")))]
compile_error!("No net selected!");

mod currency_macro;
mod lease;
mod lpn;
mod native;
mod payment;
mod symbols_macro;

#[cfg(feature = "testing")]
pub mod test;

#[cfg(test)]
mod test_impl;

pub use lease::LeaseGroup;
pub use lpn::Lpns;
pub use native::{Native, Nls};
pub use payment::PaymentGroup;
