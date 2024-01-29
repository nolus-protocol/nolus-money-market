#[cfg(all(feature = "astroport", feature = "main"))]
pub type Impl = astroport::RouterImpl<astroport::Main>;
#[cfg(all(feature = "astroport", feature = "test"))]
pub type Impl = astroport::RouterImpl<astroport::Test>;
#[cfg(feature = "osmosis")]
pub use self::osmosis::*;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis")]
mod osmosis;
