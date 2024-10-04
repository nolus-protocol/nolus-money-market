#[cfg(feature = "dex-astroport_main")]
use self::astroport as impl_mod;
#[cfg(feature = "dex-astroport_test")]
use self::astroport as impl_mod;
#[cfg(feature = "dex-test_impl")]
use self::astroport as impl_mod;
#[cfg(feature = "dex-osmosis")]
use self::osmosis as impl_mod;

#[cfg(any(
    feature = "dex-astroport_main",
    feature = "dex-astroport_test",
    feature = "dex-test_impl"
))]
mod astroport;
#[cfg(feature = "dex-osmosis")]
mod osmosis;
#[cfg(any(feature = "testing", test))]
pub mod testing;

pub type Impl = impl_mod::Impl;
