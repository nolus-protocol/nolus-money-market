use currency::Group;
use finance::coin::{CoinDTO, NonZeroAmount};
#[cfg(feature = "astroport")]
use sdk::cosmos_sdk_proto::{cosmos::base::v1beta1::Coin as ProtoCoin, prost::Name};
use sdk::cosmos_sdk_proto::{prost::Message, Any};

#[cfg(feature = "astroport")]
pub(crate) fn parse_request_from_any<T>(request: Any) -> T
where
    T: Message + Default + Name,
{
    request.to_msg().expect("Expected a swap request message!")
}

#[cfg(feature = "osmosis")]
pub(crate) fn parse_request_from_any_and_type_url<T>(request: Any, type_url: &str) -> T
where
    T: Message + Default,
{
    assert_eq!(
        request.type_url, type_url,
        "Different type URL than expected one encountered!"
    );

    T::decode(request.value.as_slice()).expect("Expected a swap request message!")
}

pub(crate) fn parse_token<G>(amount: &str, denom: String) -> CoinDTO<G>
where
    G: Group,
{
    finance::coin::from_amount_ticker(
        amount
            .parse::<NonZeroAmount>()
            .expect("Expected swap-in amount to be a non-zero unsigned integer!")
            .get(),
        denom,
    )
    .expect("Expected swap-in token to be part of selected group!")
}

#[cfg(feature = "astroport")]
pub(crate) fn parse_one_token_from_vec<G>(funds: Vec<ProtoCoin>) -> CoinDTO<G>
where
    G: Group,
{
    if let [token_in] = funds.as_slice() {
        parse_token(&token_in.amount, token_in.denom.clone())
    } else {
        unimplemented!("Expected only one type of token!");
    }
}

#[cold]
pub(crate) fn pattern_match_else(message_name: &str) -> ! {
    unimplemented!(
        r#"Expected "{message_name}" message symmetric to the one built by the "build_request" method!"#
    );
}
