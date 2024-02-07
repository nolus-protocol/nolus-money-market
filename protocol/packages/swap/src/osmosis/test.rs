use currency::{
    test::{SuperGroup, SuperGroupTestC1},
    Currency as _, SymbolStatic,
};
use dex::swap::Error;
use finance::coin::Coin;
use sdk::cosmwasm_std::Coin as CwCoin;

use super::{SwapAmountInRoute, SwapTarget};

const INVALID_TICKER: SymbolStatic = "NotATicker";

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
        Err(Error::Currency(_))
    ));
}
