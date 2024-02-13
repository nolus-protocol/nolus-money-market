#[cfg(feature = "osmosis")]
pub use self::osmosis::Impl;

#[cfg(all(feature = "astroport", feature = "main"))]
pub type Impl = astroport::RouterImpl<astroport::Main>;
#[cfg(all(feature = "astroport", feature = "test"))]
pub type Impl = astroport::RouterImpl<astroport::Test>;
#[cfg(all(feature = "astroport", feature = "migration"))]
pub use astroport::migration;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis")]
mod osmosis;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
