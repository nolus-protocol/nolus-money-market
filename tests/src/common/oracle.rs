use currency::{
    lease::{Atom, Cro, Osmo},
    lpn::Usdc,
    native::Nls,
    Currency,
};
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent,
    price::{self, Price},
};
use marketprice::{config::Config as PriceConfig, SpotPrice};
use oracle::{
    contract::{execute, instantiate, query, reply, sudo},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg},
    state::config::Config,
    ContractError,
};
use sdk::{
    cosmwasm_std::{to_binary, wasm_execute, Addr, Binary, Deps, Env, Event},
    cw_multi_test::AppResponse,
    testing::{CwContract, CwContractWrapper},
};

use crate::common::test_case::app::App;

use super::{test_case::TestCase, ADMIN};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate_default<BaseC>(app: &mut App) -> Addr
    where
        BaseC: Currency,
    {
        let endpoints: CwContractWrapper<_, _, _, _, _, _, _, _, _, _, _> =
            CwContractWrapper::new(execute, instantiate, query)
                .with_reply(reply)
                .with_sudo(sudo);

        Self::instantiate::<BaseC>(app, Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate<BaseC>(app: &mut App, endpoints: Box<CwContract>) -> Addr
    where
        BaseC: Currency,
    {
        let code_id = app.store_code(endpoints);
        let msg = InstantiateMsg {
            config: Config {
                base_asset: BaseC::TICKER.into(),
                price_config: PriceConfig::new(
                    Percent::from_percent(1),
                    Duration::from_secs(5),
                    12,
                    Percent::from_percent(75),
                ),
            },
            swap_tree: oracle::swap_tree!((1, Osmo::TICKER), (3, Cro::TICKER), (13, Atom::TICKER)),
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

pub(crate) fn mock_oracle_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    let price = price::total_of(Coin::<Nls>::new(123456789)).is(Coin::<Usdc>::new(100000000));
    let res = match msg {
        QueryMsg::Prices {} => to_binary(&oracle::msg::PricesResponse {
            prices: vec![price.into()],
        }),
        QueryMsg::Price { currency: _ } => to_binary(&SpotPrice::from(price)),
        _ => Ok(query(deps, env, msg)?),
    }?;

    Ok(res)
}

pub(crate) fn add_feeder<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>,
    addr: impl Into<String>,
) {
    let oracle = test_case.address_book.oracle().clone();

    let response: AppResponse = test_case
        .app
        .sudo(
            oracle,
            &SudoMsg::RegisterFeeder {
                feeder_address: addr.into(),
            },
        )
        .unwrap()
        .unwrap_response();

    assert!(response.data.is_none());

    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract2")],
    );
}

pub(crate) fn feed_a_price<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms, C1, C2>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>,
    addr: Addr,
    price: Price<C1, C2>,
) -> AppResponse
where
    C1: Currency,
    C2: Currency,
{
    let oracle = test_case.address_book.oracle().clone();

    test_case
        .app
        .execute_raw(
            addr,
            wasm_execute(
                oracle,
                &ExecuteMsg::FeedPrices {
                    prices: vec![price.into()],
                },
                vec![],
            )
            .unwrap(),
        )
        .expect("Oracle not properly connected!")
        .unwrap_response()
}

pub(crate) fn feed_price<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms, C1, C2>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>,
    addr: Addr,
    base: Coin<C1>,
    quote: Coin<C2>,
) -> AppResponse
where
    C1: Currency,
    C2: Currency,
{
    feed_a_price(test_case, addr, price::total_of(base).is(quote))
}
