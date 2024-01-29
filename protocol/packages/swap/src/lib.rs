#[cfg(feature = "astroport_main")]
pub type Impl = astroport::RouterImpl<astroport::Main>;
#[cfg(feature = "astroport_test")]
pub type Impl = astroport::RouterImpl<astroport::Test>;
#[cfg(feature = "osmosis")]
pub use self::osmosis::*;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis")]
mod osmosis;
mod type_url;
