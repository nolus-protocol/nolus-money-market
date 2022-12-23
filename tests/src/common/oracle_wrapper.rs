use trees::tr;

use currency::{
    lease::{Cro, Osmo},
    lpn::Usdc,
    native::Nls,
};
use finance::{
    coin::Coin, currency::Currency, duration::Duration, percent::Percent, price::total_of,
};
use marketprice::config::Config as PriceConfig;
use marketprice::SpotPrice;
use oracle::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{config::Config, supported_pairs::TreeStore},
    ContractError,
};
use sdk::{
    cosmwasm_std::{to_binary, Addr, Binary, Deps, Empty, Env},
    cw_multi_test::Executor,
};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

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
            swap_tree: TreeStore(
                tr((0, Usdc::TICKER.into()))
                    / tr((1, Osmo::TICKER.to_string()))
                    / tr((3, Cro::TICKER.to_string())),
            ),
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
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_oracle_query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let price = total_of(Coin::<Nls>::new(123456789)).is(Coin::<Usdc>::new(100000000));
    let res = match msg {
        QueryMsg::Prices { currencies: _ } => to_binary(&oracle::msg::PricesResponse {
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
    Empty,
    anyhow::Error,
    ContractError,
>;
