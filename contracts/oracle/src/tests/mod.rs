#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod oracle_tests;

use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    MemoryStorage, MessageInfo, OwnedDeps,
};
use currency::{
    lpn::Usdc,
    native::Nls,
    test::{TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD},
};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    percent::Percent,
    price::{self, dto::PriceDTO},
};

use crate::{
    contract::{execute, instantiate},
    msg::{ExecuteMsg, InstantiateMsg},
    state::supported_pairs::ResolutionPath,
};

pub(crate) const CREATOR: &str = "creator";

pub(crate) fn dummy_instantiate_msg(
    base_asset: SymbolOwned,
    price_feed_period_secs: u32,
    feeders_percentage_needed: Percent,
    currency_paths: Vec<ResolutionPath>,
    alarms_addr: String,
) -> InstantiateMsg {
    InstantiateMsg {
        base_asset,
        price_feed_period_secs,
        feeders_percentage_needed,
        currency_paths,
        timealarms_addr: alarms_addr,
    }
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg {
    dummy_instantiate_msg(
        Usdc::SYMBOL.to_string(),
        60,
        Percent::from_percent(50),
        vec![
            vec![
                TestCurrencyA::SYMBOL.to_string(),
                TestCurrencyB::SYMBOL.to_string(),
                TestCurrencyC::SYMBOL.to_string(),
                Usdc::SYMBOL.to_string(),
            ],
            vec![TestCurrencyD::SYMBOL.to_string(), Usdc::SYMBOL.to_string()],
            vec![
                Nls::SYMBOL.to_string(),
                TestCurrencyD::SYMBOL.to_string(),
                Usdc::SYMBOL.to_string(),
            ],
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
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyB>::new(10)).is(Coin::<
                TestCurrencyC,
            >::new(
                32
            )))
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<TestCurrencyC>::new(10)).is(Coin::<Usdc>::new(12)),
            )
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<TestCurrencyD>::new(10)).is(Coin::<Usdc>::new(120)),
            )
            .unwrap(),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR, &coins(1000, Nls::SYMBOL));
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // register single feeder address
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: CREATOR.to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    (deps, info)
}
