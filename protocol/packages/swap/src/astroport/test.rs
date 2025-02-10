use currency::{
    test::{SubGroupTestC10, SubGroupTestC6, SuperGroup, SuperGroupTestC1},
    CurrencyDef as _,
};
use finance::coin::Coin;
use oracle::api::swap::SwapTarget;
use sdk::cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;

use crate::testing;

use super::api::{AssetInfo, SwapOperation};

#[test]
fn to_dex_cwcoin() {
    let coin_amount = 3541415;
    let coin: Coin<SuperGroupTestC1> = coin_amount.into();
    assert_eq!(
        ProtoCoin {
            denom: SuperGroupTestC1::dex().into(),
            amount: coin_amount.to_string(),
        },
        super::to_dex_proto_coin::<SuperGroup>(&coin.into()).unwrap()
    );
}

#[test]
fn to_operations() {
    type StartSwapCurrency = SubGroupTestC10;
    let path = vec![
        SwapTarget {
            pool_id: 2,
            target: currency::dto::<SuperGroupTestC1, _>(),
        },
        SwapTarget {
            pool_id: 12,
            target: currency::dto::<SubGroupTestC6, _>(),
        },
    ];
    let expected = vec![
        SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: StartSwapCurrency::dex().into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: SuperGroupTestC1::dex().into(),
            },
        },
        SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: SuperGroupTestC1::dex().into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: SubGroupTestC6::dex().into(),
            },
        },
    ];
    assert_eq!(
        super::to_operations::<SuperGroup>(StartSwapCurrency::dex(), &path[0..0]),
        vec![]
    );
    assert_eq!(
        expected[0..1].to_vec(),
        super::to_operations::<SuperGroup>(StartSwapCurrency::dex(), &path[0..1])
    );
    assert_eq!(
        expected,
        super::to_operations::<SuperGroup>(StartSwapCurrency::dex(), &path)
    );
}

#[test]
fn validate_a_response() {
    let resp_base64 = "EksKLC9jb3Ntd2FzbS53YXNtLnYxLk1zZ0V4ZWN1dGVDb250cmFjdFJlc3BvbnNlEhsKGXsicmV0dXJuX2Ftb3VudCI6IjM4OTA4In0SSwosL2Nvc213YXNtLndhc20udjEuTXNnRXhlY3V0ZUNvbnRyYWN0UmVzcG9uc2USGwoZeyJyZXR1cm5fYW1vdW50IjoiNzIyNTUifQ==";
    let exp_amount1 = 38908;
    let exp_amount2 = 72255;

    testing::validate_a_response(resp_base64, exp_amount1, exp_amount2)
}
