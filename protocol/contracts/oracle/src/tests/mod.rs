use currencies::test::{NativeC, PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7, StableC1};
use currency::{Currency, Group, SymbolOwned};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    percent::Percent,
    price::{self, base::BasePrice, dto::PriceDTO},
};
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
        MemoryStorage, MessageInfo, OwnedDeps,
    },
};
use swap::{SwapGroup, SwapTarget};
use tree::HumanReadableTree;

use crate::{
    api::{Config, ExecuteMsg, InstantiateMsg, SudoMsg},
    contract::{instantiate, sudo},
};

#[cfg(test)]
mod oracle_tests;

pub(crate) const CREATOR: &str = "creator";

pub(crate) type TheCurrency = StableC1;

pub(crate) fn dto_price<C, G, Q, LpnG>(total_of: Amount, is: Amount) -> PriceDTO<G, LpnG>
where
    C: Currency,
    G: Group,
    Q: Currency,
    LpnG: Group,
{
    price::total_of(Coin::<C>::new(total_of))
        .is(Coin::<Q>::new(is))
        .into()
}

pub(crate) fn base_price<C>(total_of: Amount, is: Amount) -> BasePrice<SwapGroup, TheCurrency>
where
    C: Currency,
{
    price::total_of(Coin::<C>::new(total_of))
        .is(Coin::new(is))
        .into()
}

pub(crate) fn dummy_instantiate_msg(
    base_asset: SymbolOwned,
    price_feed_period_secs: u32,
    expected_feeders: Percent,
    swap_tree: HumanReadableTree<SwapTarget>,
) -> InstantiateMsg {
    InstantiateMsg {
        config: Config {
            base_asset,
            price_config: PriceConfig::new(
                expected_feeders,
                Duration::from_secs(price_feed_period_secs),
                1,
                Percent::from_percent(88),
            ),
        },
        swap_tree,
    }
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg {
    dummy_instantiate_msg(
        StableC1::TICKER.to_string(),
        60,
        Percent::from_percent(50),
        sdk::cosmwasm_std::from_json(format!(
            r#"{{
                "value":[0,"{usdc}"],
                "children":[
                    {{
                        "value":[3,"{weth}"],
                        "children":[
                            {{
                                "value":[2,"{atom}"],
                                "children":[
                                    {{"value":[1,"{osmo}"]}}
                                ]
                            }}
                        ]
                    }},
                    {{
                        "value":[4,"{axl}"],
                        "children":[
                            {{"value":[5,"{cro}"]}}
                        ]
                    }}
                ]
            }}"#,
            usdc = StableC1::TICKER,
            weth = PaymentC7::TICKER,
            atom = PaymentC3::TICKER,
            osmo = PaymentC5::TICKER,
            axl = PaymentC4::TICKER,
            cro = PaymentC6::TICKER,
        ))
        .unwrap(),
    )
}

pub(crate) fn dummy_feed_prices_msg() -> ExecuteMsg {
    ExecuteMsg::FeedPrices {
        prices: vec![
            PriceDTO::try_from(
                price::total_of(Coin::<PaymentC5>::new(10)).is(Coin::<PaymentC3>::new(12)),
            )
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<PaymentC3>::new(10)).is(Coin::<PaymentC7>::new(32)),
            )
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<PaymentC7>::new(10)).is(Coin::<StableC1>::new(12)),
            )
            .unwrap(),
            PriceDTO::try_from(
                price::total_of(Coin::<PaymentC4>::new(10)).is(Coin::<StableC1>::new(120)),
            )
            .unwrap(),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR, &coins(1000, NativeC::TICKER));
    let res: CwResponse = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert!(res.messages.is_empty());

    // register single feeder address
    let CwResponse {
        messages,
        attributes,
        events,
        data,
        ..
    }: CwResponse = sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::RegisterFeeder {
            feeder_address: CREATOR.to_string(),
        },
    )
    .unwrap();

    assert!(messages.is_empty());
    assert!(attributes.is_empty());
    assert!(events.is_empty());
    assert!(data.is_none());

    (deps, info)
}
