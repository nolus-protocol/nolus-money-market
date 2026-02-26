use currency::{
    CurrencyDef as _,
    test::{SuperGroup, SuperGroupTestC1},
};
use finance::{coin::Coin, fraction::Unit as _};
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::testing;

use super::{SwapAmountInRoute, SwapTarget};

#[test]
fn to_dex_cwcoin() {
    let coin: Coin<SuperGroupTestC1> = Coin::new(3541415);
    assert_eq!(
        CwCoin::new(coin.to_primitive(), SuperGroupTestC1::dex()),
        super::to_dex_cwcoin::<SuperGroup>(&coin.into())
    );
}

#[test]
fn into_route() {
    let path = vec![SwapTarget {
        pool_id: 2,
        target: currency::dto::<SuperGroupTestC1, _>(),
    }];
    let expected = vec![SwapAmountInRoute {
        pool_id: 2,
        token_out_denom: SuperGroupTestC1::dex().into(),
    }];
    assert_eq!(expected, super::to_route::<SuperGroup>(&path));
}

#[test]
fn validate_a_response() {
    let resp_base64 = "EkUKOS9vc21vc2lzLnBvb2xtYW5hZ2VyLnYxYmV0YTEuTXNnU3dhcEV4YWN0QW1vdW50SW5SZXNwb25zZRIICgY1MTY1NTkSRQo5L29zbW9zaXMucG9vbG1hbmFnZXIudjFiZXRhMS5Nc2dTd2FwRXhhY3RBbW91bnRJblJlc3BvbnNlEggKBjk1OTMxOQ==";
    let exp_amount1 = 516559;
    let exp_amount2 = 959319;

    testing::validate_a_response(resp_base64, exp_amount1, exp_amount2)
}
