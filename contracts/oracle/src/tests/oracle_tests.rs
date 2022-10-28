use std::collections::HashSet;

use currency::{
    lease::{Atom, Osmo, Wbtc, Weth},
    lpn::Usdc,
};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolStatic},
    price,
    price::dto::PriceDTO,
};
use sdk::cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info},
};

use crate::{
    contract::{execute, query},
    msg::{ExecuteMsg, PricesResponse, QueryMsg},
    tests::{dummy_default_instantiate_msg, setup_test},
    ContractError,
};

use super::dummy_feed_prices_msg;

#[test]
fn feed_prices_unknown_feeder() {
    let (mut deps, _) = setup_test(dummy_default_instantiate_msg());

    let msg = dummy_feed_prices_msg();
    let info = mock_info("test", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnknownFeeder {}, err)
}

#[test]
fn feed_direct_price() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let expected_price =
        PriceDTO::try_from(price::total_of(Coin::<Wbtc>::new(10)).is(Coin::<Usdc>::new(120)))
            .unwrap();

    // Feed direct price Wbtc/OracleBaseAsset
    let msg = ExecuteMsg::FeedPrices {
        prices: vec![expected_price.clone()],
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query price for Osmo
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Prices {
            currencies: HashSet::from([Wbtc::TICKER.to_string()]),
        },
    )
    .unwrap();
    let value: PricesResponse = from_binary(&res).unwrap();
    assert_eq!(expected_price, value.prices.first().unwrap().to_owned());
}

#[test]
fn feed_indirect_price() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let price_a_to_b =
        PriceDTO::try_from(price::total_of(Coin::<Osmo>::new(10)).is(Coin::<Atom>::new(120)))
            .unwrap();
    let price_b_to_c =
        PriceDTO::try_from(price::total_of(Coin::<Atom>::new(10)).is(Coin::<Weth>::new(5)))
            .unwrap();
    let price_c_to_usdc =
        PriceDTO::try_from(price::total_of(Coin::<Weth>::new(10)).is(Coin::<Usdc>::new(5)))
            .unwrap();

    // Feed indirect price from Osmo to OracleBaseAsset
    let msg = ExecuteMsg::FeedPrices {
        prices: vec![price_a_to_b, price_b_to_c, price_c_to_usdc],
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query price for Osmo
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            currency: Osmo::TICKER.to_string(),
        },
    )
    .unwrap();

    let expected_price =
        PriceDTO::try_from(price::total_of(Coin::<Osmo>::new(1)).is(Coin::<Usdc>::new(3))).unwrap();
    let value: PriceDTO = from_binary(&res).unwrap();
    assert_eq!(expected_price, value)
}

#[test]
#[should_panic(expected = "UnsupportedCurrency")]
fn query_prices_unsupported_denom() {
    let (deps, _) = setup_test(dummy_default_instantiate_msg());

    // query for unsupported denom should fail
    query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Prices {
            currencies: HashSet::from(["dummy".to_string()]),
        },
    )
    .unwrap();
}

#[test]
fn feed_prices_unsupported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct X;
    impl Currency for X {
        const TICKER: SymbolStatic = "X";
        const BANK_SYMBOL: SymbolStatic = "ibc/X";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_X";
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct C;
    impl Currency for C {
        const TICKER: SymbolStatic = "C";
        const BANK_SYMBOL: SymbolStatic = "ibc/C";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_C";
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct D;
    impl Currency for D {
        const TICKER: SymbolStatic = "D";
        const BANK_SYMBOL: SymbolStatic = "ibc/D";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_D";
    }

    let prices = vec![
        PriceDTO::try_from(price::total_of(Coin::<X>::new(10)).is(Coin::<C>::new(12))).unwrap(),
        PriceDTO::try_from(price::total_of(Coin::<X>::new(10)).is(Coin::<D>::new(22))).unwrap(),
    ];

    let msg = ExecuteMsg::FeedPrices { prices };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnsupportedDenomPairs {}, err);
}
