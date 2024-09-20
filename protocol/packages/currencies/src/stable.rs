#[cfg(not(feature = "testing"))]
include!(concat!(env!("OUT_DIR"), "/stable.rs"));

#[cfg(feature = "testing")]
pub type Stable = crate::lpn::impl_mod::Lpn;
