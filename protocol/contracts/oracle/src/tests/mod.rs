use currencies::{
    LeaseGroup as AlarmCurrencies, Lpn as BaseCurrency, Lpns as BaseCurrencies, Nls,
    PaymentGroup as PriceCurrencies,
    testing::{PaymentC1, PaymentC3, PaymentC4, PaymentC5},
};
use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    percent::Percent100,
    price::{self, Price, base::BasePrice, dto::PriceDTO},
};
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        MemoryStorage, MessageInfo, OwnedDeps, coins,
        testing::{self, MockApi, MockQuerier},
    },
    testing as sdk_testing,
};
use tree::HumanReadableTree;

use crate::{
    api::{Config, ExecuteMsg, InstantiateMsg, SudoMsg, swap::SwapTarget},
    contract, test_tree,
};

mod oracle_tests;

pub(crate) const CREATOR: &str = "creator";

pub(crate) fn dto_price<C, G, Q>(total_of: Amount, is: Amount) -> PriceDTO<G>
where
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    G: Group<TopG = G>,
    Q: CurrencyDef,
    Q::Group: MemberOf<G>,
{
    test_price::<C, Q>(total_of, is).into()
}

pub(crate) fn base_price<C>(
    total_of: Amount,
    is: Amount,
) -> BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>
where
    C: CurrencyDef,
    C::Group: MemberOf<PriceCurrencies>,
{
    test_price::<C, BaseCurrency>(total_of, is).into()
}

pub(crate) fn dummy_instantiate_msg(
    price_feed_period_secs: u32,
    expected_feeders: Percent100,
    swap_tree: HumanReadableTree<SwapTarget<PriceCurrencies>>,
) -> InstantiateMsg<PriceCurrencies> {
    InstantiateMsg {
        config: Config {
            price_config: PriceConfig::new(
                expected_feeders,
                Duration::from_secs(price_feed_period_secs),
                1,
                Percent100::from_percent(88),
            ),
        },
        swap_tree,
    }
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg<PriceCurrencies> {
    dummy_instantiate_msg(
        60,
        Percent100::from_percent(50),
        test_tree::dummy_swap_tree(),
    )
}

pub(crate) fn dummy_feed_prices_msg()
-> ExecuteMsg<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies> {
    ExecuteMsg::FeedPrices {
        prices: vec![
            test_price::<PaymentC3, PaymentC5>(10, 12).into(),
            test_price::<PaymentC5, PaymentC4>(10, 32).into(),
            test_price::<PaymentC4, BaseCurrency>(10, 12).into(),
            test_price::<PaymentC1, BaseCurrency>(10, 120).into(),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg<PriceCurrencies>,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = testing::mock_dependencies();

    let info = MessageInfo {
        sender: sdk_testing::user(CREATOR),
        funds: coins(1000, Nls::ticker()),
    };

    let res: CwResponse =
        contract::instantiate(deps.as_mut(), testing::mock_env(), info.clone(), msg)
            .expect("Contract should be instantiatable");
    assert!(res.messages.is_empty());

    // register single feeder address
    let CwResponse {
        messages,
        attributes,
        events,
        data,
        ..
    }: CwResponse = contract::sudo(
        deps.as_mut(),
        testing::mock_env(),
        SudoMsg::RegisterFeeder {
            feeder_address: sdk_testing::user(CREATOR).to_string(),
        },
    )
    .expect("Sudo endpoint should be able to register feeder");

    assert!(messages.is_empty());
    assert!(attributes.is_empty());
    assert!(events.is_empty());
    assert!(data.is_none());

    (deps, info)
}

pub(crate) fn test_price<C, Q>(total_of: Amount, is: Amount) -> Price<C, Q>
where
    C: CurrencyDef,
    Q: CurrencyDef,
{
    price::total_of(Coin::<C>::new(total_of)).is(Coin::<Q>::new(is))
}
