use cosmwasm_std::{Addr, Binary, Deps, Env, StdError, StdResult, to_binary};
use cw_multi_test::{App, Executor};

use marketprice::storage::Price;
use oracle::{
    contract::{execute, instantiate, query, reply},
    ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg}
};

use crate::common::ContractWrapper;

use super::{ADMIN, NATIVE_DENOM};

pub struct MarketOracleWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            ExecuteMsg,
            ContractError,
            InstantiateMsg,
            ContractError,
            QueryMsg,
            StdError,
            cosmwasm_std::Empty,
            anyhow::Error,
            ContractError,
        >,
    >,
}

impl MarketOracleWrapper {
    pub fn with_contract_wrapper(
        contract: ContractWrapper<
            ExecuteMsg,
            ContractError,
            InstantiateMsg,
            ContractError,
            QueryMsg,
            StdError,
            cosmwasm_std::Empty,
            anyhow::Error,
            ContractError,
        >,
    ) -> Self {
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
        let contract = ContractWrapper::new(
            execute,
            instantiate,
            query,
        ).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_oracle_query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let res = match msg {
        QueryMsg::PriceFor { denoms: _ } => to_binary(&oracle::msg::PriceResponse {
            prices: vec![Price::new(NATIVE_DENOM, 123456789, "UST", 1000000000)],
        }),
        _ => Ok(query(deps, env, msg)?),
    }?;

    Ok(res)
}
