#[cfg(any(feature = "dex-astroport_main", feature = "dex-astroport_test"))]
mod astroport;
#[cfg(feature = "dex-osmosis")]
mod osmosis;
#[cfg(any(feature = "testing", test))]
pub mod testing;

pub type Impl = PrivateImpl;

#[cfg(feature = "dex-astroport_main")]
type PrivateImpl = astroport::Impl<astroport::NeutronMain>;
#[cfg(feature = "dex-astroport_test")]
type PrivateImpl = astroport::Impl<astroport::NeutronTest>;
#[cfg(feature = "dex-osmosis")]
type PrivateImpl = osmosis::Impl;
