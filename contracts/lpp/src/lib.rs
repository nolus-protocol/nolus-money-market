pub mod msg;

#[cfg(not(feature = "library"))]
#[path=""]
mod nolib {
    pub mod contract;
    pub mod error;
    pub mod state;
    pub mod config;
    pub mod loan;
}

#[cfg(not(feature = "library"))]
pub use nolib::*;
