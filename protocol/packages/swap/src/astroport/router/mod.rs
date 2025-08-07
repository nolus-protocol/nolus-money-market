#[cfg(feature = "dex-astroport_main")]
mod main;
#[cfg(any(feature = "dex-astroport_test", feature = "dex-test_impl",))]
mod test;

#[cfg(feature = "dex-astroport_main")]
pub type Impl = main::MainRouter;
#[cfg(feature = "dex-astroport_test")]
pub type Impl = test::TestRouter;
#[cfg(feature = "dex-test_impl")]
pub type Impl = test::TestRouter;

pub trait Router {
    const ADDRESS: &'static str;
}
