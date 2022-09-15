#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod oracle_tests;

use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    MemoryStorage, MessageInfo, OwnedDeps,
};
use finance::{
    coin::Coin,
    currency::{
        Currency, Nls, SymbolOwned, TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD,
        Usdc,
    },
    price::{self, dto::PriceDTO},
};

use crate::{
    contract::{execute, instantiate},
    msg::{ExecuteMsg, InstantiateMsg},
};

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
        Usdc::SYMBOL.to_string(),
        60,
        50,
        vec![
            (
                TestCurrencyA::SYMBOL.to_string(),
                TestCurrencyB::SYMBOL.to_string(),
            ),
            (
                TestCurrencyA::SYMBOL.to_string(),
                TestCurrencyC::SYMBOL.to_string(),
            ),
            (
                TestCurrencyB::SYMBOL.to_string(),
                TestCurrencyC::SYMBOL.to_string(),
            ),
            (
                TestCurrencyC::SYMBOL.to_string(),
                TestCurrencyD::SYMBOL.to_string(),
            ),
            (TestCurrencyA::SYMBOL.to_string(), Usdc::SYMBOL.to_string()),
            (TestCurrencyB::SYMBOL.to_string(), Usdc::SYMBOL.to_string()),
            (TestCurrencyC::SYMBOL.to_string(), Usdc::SYMBOL.to_string()),
            (Nls::SYMBOL.to_string(), TestCurrencyD::SYMBOL.to_string()),
        ],
        "timealarms".to_string(),
    )
}

pub(crate) fn dummy_feed_prices_msg() -> ExecuteMsg {
    ExecuteMsg::FeedPrices {
        prices: vec![
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyA>::new(10)).is(Coin::<
                TestCurrencyB,
            >::new(
                12
            )))
            .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyA>::new(10)).is(Coin::<
                TestCurrencyC,
            >::new(
                32
            )))
            .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyC>::new(10)).is(Coin::<
                TestCurrencyD,
            >::new(
                12
            )))
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<TestCurrencyA>::new(10)).is(Coin::<Usdc>::new(120)),
            )
            .unwrap(),
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
