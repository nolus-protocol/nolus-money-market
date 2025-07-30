use currencies::{
    LeaseGroup as AlarmCurrencies, Lpn as BaseCurrency, Lpns as BaseCurrencies, Nls,
    PaymentGroup as PriceCurrencies,
};
use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent100,
    price::{self, Price, base::BasePrice},
};
use marketprice::config::Config as PriceConfig;
use oracle::{
    api::{Config, ExecuteMsg, InstantiateMsg, PricesResponse, QueryMsg, SudoMsg},
    contract::{execute, instantiate, query, reply, sudo},
    error::Error,
    result::Result,
    test_tree,
};
use sdk::{
    cosmwasm_std::{
        Addr, Binary, Deps, Empty, Env, Event, QuerierWrapper, StdResult, to_json_binary,
        wasm_execute,
    },
    cw_multi_test::AppResponse,
    testing::{self, CwContract, CwContractWrapper},
};
use serde::Serialize;

use super::{
    ADMIN,
    test_case::{TestCase, app::App, response::ResponseWithInterChainMsgs},
};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate_default(app: &mut App) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        Self::instantiate(app, Box::new(endpoints), None)
    }

    #[track_caller]
    pub fn instantiate(app: &mut App, endpoints: Box<CwContract>, admin: Option<Addr>) -> Addr {
        let code_id = app.store_code(endpoints);
        let msg = InstantiateMsg {
            config: Config {
                price_config: PriceConfig::new(
                    Percent100::from_percent(1),
                    Duration::from_secs(5),
                    12,
                    Percent100::from_percent(75),
                ),
            },

            swap_tree: test_tree::dummy_swap_tree(),
        };

        app.instantiate(
            code_id,
            testing::user(ADMIN),
            &msg,
            &Vec::default(),
            "oracle",
            admin.map(Addr::into_string),
        )
        .unwrap()
        .unwrap_response()
    }
}

pub(crate) fn mock_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg<PriceCurrencies>,
) -> Result<Binary, PriceCurrencies> {
    let price =
        price::total_of(Coin::<Nls>::new(123456789)).is(Coin::<BaseCurrency>::new(100000000));

    match msg {
        QueryMsg::Prices {} => {
            to_json_binary(
                &PricesResponse::<PriceCurrencies, BaseCurrency, BaseCurrencies> {
                    prices: vec![price.into()],
                },
            )
            .map_err(Error::ConvertToBinary)
        }
        _ => query(deps, env, msg),
    }
}

pub(crate) fn add_feeder<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
    feeder: Addr,
) {
    let oracle = test_case.address_book.oracle().clone();

    let response: AppResponse = test_case
        .app
        .sudo(
            oracle.clone(),
            &SudoMsg::<PriceCurrencies>::RegisterFeeder {
                feeder_address: feeder.into(),
            },
        )
        .unwrap()
        .unwrap_response();

    assert!(response.data.is_none());

    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", oracle)],
    );
}

pub(crate) fn feed_price_pair<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    TimeAlarms,
    C1,
    C2,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
    sender: Addr,
    price: Price<C1, C2>,
) -> AppResponse
where
    C1: CurrencyDef,
    C1::Group: MemberOf<PriceCurrencies>,
    C2: CurrencyDef,
    C2::Group: MemberOf<PriceCurrencies>,
{
    let oracle = test_case.address_book.oracle().clone();

    test_case
        .app
        .execute_raw(
            sender,
            wasm_execute(
                oracle,
                &ExecuteMsg::<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>::FeedPrices {
                    prices: vec![price.into()],
                },
                vec![],
            )
            .unwrap(),
        )
        .expect("Oracle not properly connected!")
        .unwrap_response()
}

pub(crate) fn feed_price<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    TimeAlarms,
    C1,
    C2,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
    sender: Addr,
    base: Coin<C1>,
    quote: Coin<C2>,
) -> AppResponse
where
    C1: CurrencyDef,
    C1::Group: MemberOf<PriceCurrencies>,
    C2: CurrencyDef,
    C2::Group: MemberOf<PriceCurrencies>,
{
    feed_price_pair(test_case, sender, price::total_of(base).is(quote))
}

pub(crate) fn dispatch<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
    sender: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    let oracle = test_case.address_book.oracle().clone();

    test_case
    .app
    .execute_raw(
        sender,
        wasm_execute(
            oracle,
            &ExecuteMsg::<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>::DispatchAlarms { max_count: 32 },
            vec![],
        )
        .unwrap(),
    )
    .expect("Oracle not properly connected!")
}

pub(crate) fn fetch_price<BaseC, BaseG, QuoteC, QuoteG>(
    querier: QuerierWrapper<'_, Empty>,
    oracle: Addr,
) -> StdResult<BasePrice<BaseG, QuoteC, QuoteG>>
where
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG>,
    BaseG: Group + Serialize,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG> + MemberOf<BaseG::TopG>,
    QuoteG: Group,
{
    querier.query_wasm_smart(
        oracle,
        &QueryMsg::<BaseG>::BasePrice {
            currency: currency::dto::<BaseC, _>(),
        },
    )
}
