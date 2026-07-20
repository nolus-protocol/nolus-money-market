use currency::{
    CurrencyDef as _,
    test::{SuperGroup, SuperGroupTestC1},
};
use finance::coin::Coin;
use sdk::cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;

use crate::testing;

#[test]
fn to_dex_cwcoin() {
    let coin_amount = 3541415;
    let coin: Coin<SuperGroupTestC1> = Coin::new(coin_amount);
    assert_eq!(
        ProtoCoin {
            denom: SuperGroupTestC1::dex().into(),
            amount: coin_amount.to_string(),
        },
        super::to_dex_proto_coin::<SuperGroup>(&coin.into())
    );
}

#[test]
fn validate_a_response() {
    let resp_base64 = "EksKLC9jb3Ntd2FzbS53YXNtLnYxLk1zZ0V4ZWN1dGVDb250cmFjdFJlc3BvbnNlEhsKGXsicmV0dXJuX2Ftb3VudCI6IjM4OTA4In0SSwosL2Nvc213YXNtLndhc20udjEuTXNnRXhlY3V0ZUNvbnRyYWN0UmVzcG9uc2USGwoZeyJyZXR1cm5fYW1vdW50IjoiNzIyNTUifQ==";
    let exp_amount1 = 38908;
    let exp_amount2 = 72255;

    testing::validate_a_response(resp_base64, exp_amount1, exp_amount2)
}
