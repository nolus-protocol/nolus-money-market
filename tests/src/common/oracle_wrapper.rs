use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env};
use cw_multi_test::{App, Executor};

use finance::{
    coin::Coin,
    currency::{Nls, Usdc},
    price::{self, PriceDTO},
};
use oracle::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

use crate::common::ContractWrapper;

use super::{ADMIN, NATIVE_DENOM};

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
    pub fn instantiate(self, app: &mut App, denom: &str, timealarms_addr: &str) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            base_asset: denom.to_string(),
            price_feed_period_secs: 60,
            feeders_percentage_needed: 1,
            supported_denom_pairs: vec![("UST".to_string(), NATIVE_DENOM.to_string())],
            timealarms_addr: timealarms_addr.to_string(),
        };
        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "oracle", None)
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
    let price = price::total_of(Coin::<Nls>::new(123456789)).is(Coin::<Usdc>::new(100000000));
    let res = match msg {
        QueryMsg::PriceFor { denoms: _ } => to_binary(&oracle::msg::PriceResponse {
            price: PriceDTO::try_from(price).unwrap(),
        }),
        QueryMsg::Price { denom: _ } => to_binary(&oracle::msg::PriceResponse {
            price: PriceDTO::try_from(price).unwrap(),
        }),
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
    cosmwasm_std::Empty,
    anyhow::Error,
    ContractError,
>;
