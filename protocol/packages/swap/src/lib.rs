#[cfg(any(
    feature = "dex-astroport_main",
    feature = "dex-astroport_test",
    feature = "dex-test_impl",
))]
mod astroport;
#[cfg(feature = "dex-osmosis")]
mod osmosis;
#[cfg(any(feature = "testing", test))]
pub mod testing;

pub type Impl = PrivateImpl;

#[cfg(any(
    feature = "dex-astroport_main",
    feature = "dex-astroport_test",
    feature = "dex-test_impl",
))]
type PrivateImpl = astroport::Impl;

#[cfg(feature = "dex-osmosis")]
type PrivateImpl = osmosis::Impl;
