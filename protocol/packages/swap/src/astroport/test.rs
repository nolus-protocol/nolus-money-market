use astroport::{asset::AssetInfo, router::SwapOperation};

use currency::{
    test::{SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC6},
    Currency as _, SymbolStatic,
};
use dex::swap::Error;
use finance::coin::Coin;
use oracle::api::swap::SwapTarget;
use sdk::{cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin, cosmwasm_std::Decimal};

use super::{Main, RouterImpl};

const INVALID_TICKER: SymbolStatic = "NotATicker";

#[test]
fn const_eq_max_allowed_slippage() {
    assert_eq!(
        RouterImpl::<Main>::MAX_IMPACT,
        astroport::pair::MAX_ALLOWED_SLIPPAGE
            .parse::<Decimal>()
            .unwrap()
    );
}

#[test]
fn to_dex_symbol() {
    type Currency = SuperGroupTestC1;
    assert_eq!(
        Ok(Currency::DEX_SYMBOL),
        super::to_dex_symbol::<SuperGroup>(Currency::TICKER)
    );
}

#[test]
fn to_dex_symbol_err() {
    assert!(matches!(
        super::to_dex_symbol::<SuperGroup>(INVALID_TICKER),
        Err(Error::Currency(_))
    ));
}

#[test]
fn to_dex_cwcoin() {
    let coin_amount = 3541415;
    let coin: Coin<SuperGroupTestC1> = coin_amount.into();
    assert_eq!(
        ProtoCoin {
            denom: SuperGroupTestC1::DEX_SYMBOL.into(),
            amount: coin_amount.to_string(),
        },
        super::to_dex_proto_coin::<SuperGroup>(&coin.into()).unwrap()
    );
}

#[test]
fn to_operations() {
    type StartSwapCurrency = SubGroupTestC1;
    let path = vec![
        SwapTarget {
            pool_id: 2,
            target: SuperGroupTestC1::TICKER.into(),
        },
        SwapTarget {
            pool_id: 12,
            target: SuperGroupTestC6::TICKER.into(),
        },
    ];
    let expected = vec![
        SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: StartSwapCurrency::DEX_SYMBOL.into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: SuperGroupTestC1::DEX_SYMBOL.into(),
            },
        },
        SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: SuperGroupTestC1::DEX_SYMBOL.into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: SuperGroupTestC6::DEX_SYMBOL.into(),
            },
        },
    ];
    assert_eq!(
        Ok(vec![]),
        super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path[0..0])
    );
    assert_eq!(
        Ok(expected[0..1].to_vec()),
        super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path[0..1])
    );
    assert_eq!(
        Ok(expected),
        super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path)
    );
}

#[test]
fn to_operations_err() {
    let path = vec![SwapTarget {
        pool_id: 2,
        target: INVALID_TICKER.into(),
    }];
    assert!(matches!(
        super::to_operations::<SuperGroup>(SuperGroupTestC1::DEX_SYMBOL, &path),
        Err(Error::Currency(_))
    ));
}
