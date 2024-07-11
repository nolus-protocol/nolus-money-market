use currency::{
    test::{SuperGroup, SuperGroupTestC1},
    Currency as _, SymbolStatic,
};
use dex::swap::Error;
use finance::coin::Coin;
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::testing;

use super::{SwapAmountInRoute, SwapTarget};

const INVALID_TICKER: SymbolStatic = "NotATicker";

#[test]
fn to_dex_cwcoin() {
    let coin: Coin<SuperGroupTestC1> = 3541415.into();
    assert_eq!(
        CwCoin::new(coin.into(), SuperGroupTestC1::DEX_SYMBOL),
        super::to_dex_cwcoin::<SuperGroup>(&coin.into()).unwrap()
    );
}

#[test]
fn into_route() {
    let path = vec![SwapTarget {
        pool_id: 2,
        target: SuperGroupTestC1::TICKER.into(),
    }];
    let expected = vec![SwapAmountInRoute {
        pool_id: 2,
        token_out_denom: SuperGroupTestC1::DEX_SYMBOL.into(),
    }];
    assert_eq!(Ok(expected), super::to_route::<SuperGroup>(&path));
}

#[test]
fn into_route_err() {
    let path = vec![SwapTarget {
        pool_id: 2,
        target: INVALID_TICKER.into(),
    }];
    assert!(matches!(
        super::to_route::<SuperGroup>(&path),
        Err(Error::Platform(_))
    ));
}

#[test]
fn validate_a_response() {
    let resp_base64 = "EkUKOS9vc21vc2lzLnBvb2xtYW5hZ2VyLnYxYmV0YTEuTXNnU3dhcEV4YWN0QW1vdW50SW5SZXNwb25zZRIICgY1MTY1NTkSRQo5L29zbW9zaXMucG9vbG1hbmFnZXIudjFiZXRhMS5Nc2dTd2FwRXhhY3RBbW91bnRJblJlc3BvbnNlEggKBjk1OTMxOQ==";
    let exp_amount1 = 516559;
    let exp_amount2 = 959319;

    testing::validate_a_response(resp_base64, exp_amount1, exp_amount2)
}
