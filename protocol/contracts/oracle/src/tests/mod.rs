use currencies::{
    LeaseGroup as AlarmCurrencies, Lpn as BaseCurrency, Lpns as BaseCurrencies, Nls, PaymentC1,
    PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7, PaymentGroup as PriceCurrencies,
};
use currency::{CurrencyDef, Group, MemberOf};
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
use tree::HumanReadableTree;

use crate::{
    api::{swap::SwapTarget, Config, ExecuteMsg, InstantiateMsg, SudoMsg},
    contract::{instantiate, sudo},
};

#[cfg(test)]
mod oracle_tests;

pub(crate) const CREATOR: &str = "creator";

pub(crate) fn dto_price<C, G, Q, LpnG>(total_of: Amount, is: Amount) -> PriceDTO<G, LpnG>
where
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    G: Group,
    Q: CurrencyDef,
    Q::Group: MemberOf<LpnG>,
    LpnG: Group,
{
    price::total_of(Coin::<C>::new(total_of))
        .is(Coin::<Q>::new(is))
        .into()
}

pub(crate) fn base_price<C>(
    total_of: Amount,
    is: Amount,
) -> BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>
where
    C: CurrencyDef,
    C::Group: MemberOf<PriceCurrencies>,
{
    price::total_of(Coin::<C>::new(total_of))
        .is(Coin::new(is))
        .into()
}

pub(crate) fn dummy_instantiate_msg(
    price_feed_period_secs: u32,
    expected_feeders: Percent,
    swap_tree: HumanReadableTree<SwapTarget<PriceCurrencies>>,
) -> InstantiateMsg<PriceCurrencies> {
    InstantiateMsg {
        config: Config {
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

pub(crate) fn dummy_swap_tree() -> HumanReadableTree<SwapTarget<PriceCurrencies>> {
    sdk::cosmwasm_std::from_json(format!(
        r#"{{
            "value":[0,"{lpn}"],
            "children":[
                {{
                    "value":[3,"{p4}"],
                    "children":[
                        {{
                            "value":[2,"{p5}"],
                            "children":[
                                {{"value":[1,"{p3}"]}}
                            ]
                        }},
                        {{"value":[15,"{p6}"]}}
                    ]
                }},
                {{
                    "value":[4,"{p1}"],
                    "children":[
                        {{"value":[5,"{p7}"]}}
                    ]
                }}
            ]
        }}"#,
        lpn = BaseCurrency::definition().dto(),
        p4 = PaymentC4::definition().dto(),
        p5 = PaymentC5::definition().dto(),
        p3 = PaymentC3::definition().dto(),
        p1 = PaymentC1::definition().dto(),
        p6 = PaymentC6::definition().dto(),
        p7 = PaymentC7::definition().dto(),
    ))
    .unwrap()
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg<PriceCurrencies> {
    dummy_instantiate_msg(60, Percent::from_percent(50), dummy_swap_tree())
}

pub(crate) fn dummy_feed_prices_msg(
) -> ExecuteMsg<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies> {
    ExecuteMsg::FeedPrices {
        prices: vec![
            PriceDTO::from(
                price::total_of(Coin::<PaymentC3>::new(10)).is(Coin::<PaymentC5>::new(12)),
            ),
            PriceDTO::from(
                price::total_of(Coin::<PaymentC5>::new(10)).is(Coin::<PaymentC4>::new(32)),
            ),
            PriceDTO::from(
                price::total_of(Coin::<PaymentC4>::new(10)).is(Coin::<BaseCurrency>::new(12)),
            ),
            PriceDTO::from(
                price::total_of(Coin::<PaymentC1>::new(10)).is(Coin::<BaseCurrency>::new(120)),
            ),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg<PriceCurrencies>,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR, &coins(1000, Nls::ticker()));
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
