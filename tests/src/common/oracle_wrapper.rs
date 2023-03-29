use currency::{
    lease::{Atom, Cro, Osmo},
    lpn::Usdc,
    native::Nls,
};
use finance::{
    coin::Coin,
    currency::Currency,
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
    cw_multi_test::{AppResponse, Executor},
};

use super::{test_case::TestCase, ContractWrapper, MockApp, ADMIN};

pub struct MarketOracleWrapper {
    contract_wrapper: Box<OracleContractWrapper>,
}

impl MarketOracleWrapper {
    pub fn with_contract_wrapper(contract: OracleContractWrapper) -> Self {
        Self {
            contract_wrapper: Box::new(contract),
        }
    }
    #[track_caller]
    pub fn instantiate<BaseC>(self, app: &mut MockApp) -> Addr
    where
        BaseC: Currency,
    {
        let code_id = app.store_code(self.contract_wrapper);
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

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &Vec::default(),
            "oracle",
            None,
        )
        .unwrap()
    }
}

impl Default for MarketOracleWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_oracle_query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
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

type OracleContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
    SudoMsg,
    ContractError,
    ContractError,
>;

pub fn add_feeder<Lpn>(test_case: &mut TestCase<Lpn>, addr: impl Into<String>)
where
    Lpn: Currency,
{
    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            test_case.oracle.clone().unwrap(),
            &SudoMsg::RegisterFeeder {
                feeder_address: addr.into(),
            },
        )
        .unwrap();

    assert_eq!(response.data, None);

    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract2")],
    );
}

pub fn feed_a_price<Lpn, C1, C2>(
    test_case: &mut TestCase<Lpn>,
    addr: &Addr,
    price: Price<C1, C2>,
) -> AppResponse
where
    Lpn: Currency,
    C1: Currency,
    C2: Currency,
{
    test_case
        .app
        .execute(
            addr.clone(),
            wasm_execute(
                test_case.oracle.clone().unwrap(),
                &ExecuteMsg::FeedPrices {
                    prices: vec![price.into()],
                },
                vec![],
            )
            .unwrap()
            .into(),
        )
        .expect("Oracle not properly connected!")
}

pub fn feed_price<Lpn, C1, C2>(
    test_case: &mut TestCase<Lpn>,
    addr: &Addr,
    base: Coin<C1>,
    quote: Coin<C2>,
) -> AppResponse
where
    Lpn: Currency,
    C1: Currency,
    C2: Currency,
{
    feed_a_price(test_case, addr, price::total_of(base).is(quote))
}
