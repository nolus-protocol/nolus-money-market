// TODO only export `Impl`
#[cfg(feature = "osmosis")]
pub use self::osmosis::*;

#[cfg(all(feature = "astroport", feature = "main"))]
pub type Impl = astroport::RouterImpl<astroport::Main>;
#[cfg(all(feature = "astroport", feature = "test"))]
pub type Impl = astroport::RouterImpl<astroport::Test>;

#[cfg(feature = "astroport")]
mod astroport;
#[cfg(feature = "osmosis")]
mod osmosis;

// #[cfg(any(test, feature = "testing"))] revert TODO report a cargo bug that 'test' cfg is not applied
#[cfg(feature = "testing")]
fn parse_token<G>(amount: &str, denom: String) -> finance::coin::CoinDTO<G>
where
    G: currency::Group,
{
    finance::coin::from_amount_ticker(
        amount
            .parse::<finance::coin::NonZeroAmount>()
            .expect("Expected swap-in amount to be a non-zero unsigned integer!")
            .get(),
        denom,
    )
    .expect("Expected swap-in token to be part of selected group!")
}

// #[cfg(any(test, feature = "testing"))] revert TODO report a cargo bug that 'test' cfg is not applied
#[cfg(feature = "testing")]
#[cold]
fn pattern_match_else(message_name: &str) -> ! {
    unimplemented!(
        r#"Expected "{message_name}" message symmetric to the one built by the "build_request" method!"#
    );
}
