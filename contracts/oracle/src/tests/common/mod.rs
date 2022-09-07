use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    MemoryStorage, MessageInfo, OwnedDeps,
};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned, SymbolStatic},
    price::{self, PriceDTO},
};
use marketprice::storage::Price;

use crate::{
    contract::{execute, instantiate},
    msg::{ExecuteMsg, InstantiateMsg},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct A;
impl Currency for A {
    const SYMBOL: SymbolStatic = "A";
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct B;
impl Currency for B {
    const SYMBOL: SymbolStatic = "B";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct C;
impl Currency for C {
    const SYMBOL: SymbolStatic = "C";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct D;
impl Currency for D {
    const SYMBOL: SymbolStatic = "D";
}

pub(crate) const CREATOR: &str = "creator";

pub(crate) fn dummy_instantiate_msg(
    base_asset: SymbolOwned,
    price_feed_period_secs: u32,
    feeders_percentage_needed: u8,
    supported_denom_pairs: Vec<(String, String)>,
    alarms_addr: String,
) -> InstantiateMsg {
    InstantiateMsg {
        base_asset,
        price_feed_period_secs,
        feeders_percentage_needed,
        supported_denom_pairs,
        timealarms_addr: alarms_addr,
    }
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg {
    dummy_instantiate_msg(
        "B".to_string(),
        60,
        50,
        vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("C".to_string(), "D".to_string()),
        ],
        "timealarms".to_string(),
    )
}

pub(crate) fn dummy_feed_prices_msg() -> ExecuteMsg {
    ExecuteMsg::FeedPrices {
        prices: vec![
            PriceDTO::try_from(price::total_of(Coin::<A>::new(10)).is(Coin::<B>::new(12))).unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<A>::new(10)).is(Coin::<C>::new(32))).unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<C>::new(10)).is(Coin::<D>::new(12))).unwrap(),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR, &coins(1000, "token"));
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // register single feeder address
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: CREATOR.to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    (deps, info)
}
