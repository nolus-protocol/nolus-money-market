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

#[cfg(feature = "testing")]
fn parse_dex_token<G>(amount: &str, denom: &str) -> finance::coin::CoinDTO<G>
where
    G: currency::Group,
{
    use std::marker::PhantomData;

    use serde::{de::DeserializeOwned, ser::Serialize};

    use currency::{AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit as _};
    use finance::coin::{Amount, Coin, CoinDTO, NonZeroAmount};

    struct ConstructDto<G>
    where
        G: Group,
    {
        amount: Amount,
        _group: PhantomData<G>,
    }
    impl<G> AnyVisitor for ConstructDto<G>
    where
        G: Group,
    {
        type Output = CoinDTO<G>;
        type Error = currency::error::Error;

        fn on<C>(self) -> AnyVisitorResult<Self>
        where
            C: Currency + Serialize + DeserializeOwned,
        {
            Ok(Coin::<C>::new(self.amount).into())
        }
    }

    currency::DexSymbols
        .visit_any::<G, _>(
            denom,
            ConstructDto {
                amount: amount
                    .parse::<NonZeroAmount>()
                    .expect("Expected swap-in amount to be a non-zero unsigned integer!")
                    .get(),
                _group: PhantomData,
            },
        )
        .expect("Expected swap-in token to be part of selected group!")
}

#[cfg(feature = "testing")]
#[cold]
fn pattern_match_else(message_name: &str) -> ! {
    unimplemented!(
        r#"Expected "{message_name}" message symmetric to the one built by the "build_request" method!"#
    );
}
