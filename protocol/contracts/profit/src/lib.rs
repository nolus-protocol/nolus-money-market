#[cfg(feature = "api")]
pub mod msg;
#[cfg(feature = "api")]
pub mod typedefs;

#[cfg(feature = "stub")]
pub mod stub;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
mod access_control;

#[cfg(all(feature = "contract", any(feature = "astroport", feature = "osmosis")))]
pub mod contract;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
pub mod error;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
pub mod profit;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
pub mod result;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
pub mod state;
