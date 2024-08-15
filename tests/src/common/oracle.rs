use currencies::{
    LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseGroup as AlarmCurrencies, Lpn as BaseCurrency,
    Lpns as BaseCurrencies, Nls, PaymentGroup as PriceCurrencies,
};
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent,
    price::{self, Price},
};
use marketprice::config::Config as PriceConfig;
use oracle::{
    api::{Config, ExecuteMsg, InstantiateMsg, PricesResponse, QueryMsg, SudoMsg},
    contract::{execute, instantiate, query, reply, sudo},
    ContractError,
};
use sdk::{
    cosmwasm_std::{to_json_binary, wasm_execute, Addr, Binary, Deps, Env, Event},
    cw_multi_test::AppResponse,
    testing::{CwContract, CwContractWrapper},
};

use super::{
    test_case::{app::App, TestCase},
    ADMIN,
};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate_default(app: &mut App) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        Self::instantiate(app, Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate(app: &mut App, endpoints: Box<CwContract>) -> Addr {
        let code_id = app.store_code(endpoints);
        let msg = InstantiateMsg {
            config: Config {
                price_config: PriceConfig::new(
                    Percent::from_percent(1),
                    Duration::from_secs(5),
                    12,
                    Percent::from_percent(75),
                ),
            },
            
            swap_tree: oracle::swap_tree!(
                { base: BaseCurrency::ticker() },
                (1, LeaseC2::ticker()),
                (3, LeaseC3::ticker()),
                (7, LeaseC4::ticker()),
                (11, Nls::ticker()),
                (13, LeaseC1::ticker()),
            ),
        };

        app.instantiate(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &Vec::default(),
            "oracle",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}

pub(crate) fn mock_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg<PriceCurrencies>,
) -> Result<Binary, ContractError> {
    let price =
        price::total_of(Coin::<Nls>::new(123456789)).is(Coin::<BaseCurrency>::new(100000000));

    match msg {
        QueryMsg::Prices {} => {
            to_json_binary(
                &PricesResponse::<PriceCurrencies, BaseCurrency, BaseCurrencies> {
                    prices: vec![price.into()],
                },
            )
            .map_err(ContractError::ConvertToBinary)
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
    addr: impl Into<String>,
) {
    let oracle = test_case.address_book.oracle().clone();

    let response: AppResponse = test_case
        .app
        .sudo(
            oracle,
            &SudoMsg::<PriceCurrencies>::RegisterFeeder {
                feeder_address: addr.into(),
            },
        )
        .unwrap()
        .unwrap_response();

    assert!(response.data.is_none());

    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", "contract2")],
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
    addr: Addr,
    base: Coin<C1>,
    quote: Coin<C2>,
) -> AppResponse
where
    C1: CurrencyDef,
    C1::Group: MemberOf<PriceCurrencies>,
    C2: CurrencyDef,
    C2::Group: MemberOf<PriceCurrencies>,
{
    feed_price_pair(test_case, addr, price::total_of(base).is(quote))
}
