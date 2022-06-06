use std::str::FromStr;

use cosmwasm_std::{to_binary, Addr, Binary, Decimal, Deps, Env, StdError, StdResult};
use cw_multi_test::{App, ContractWrapper, Executor};
use marketprice::feed::{DenomToPrice, Price};

use super::ADMIN;

pub struct MarketOracleWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            oracle::msg::ExecuteMsg,
            oracle::msg::InstantiateMsg,
            oracle::msg::QueryMsg,
            oracle::ContractError,
            oracle::ContractError,
            StdError,
        >,
    >,
}

impl MarketOracleWrapper {
    pub fn with_contract_wrapper(
        contract: ContractWrapper<
            oracle::msg::ExecuteMsg,
            oracle::msg::InstantiateMsg,
            oracle::msg::QueryMsg,
            oracle::ContractError,
            oracle::ContractError,
            StdError,
        >,
    ) -> Self {
        Self {
            contract_wrapper: Box::new(contract),
        }
    }
    #[track_caller]
    pub fn instantiate(self, app: &mut App, denom: &str) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = oracle::msg::InstantiateMsg {
            base_asset: denom.to_string(),
            price_feed_period: 60,
            feeders_percentage_needed: 1,
            supported_denom_pairs: vec![("UST".to_string(), "unolus".to_string())],
        };
        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "oracle", None)
            .unwrap()
    }
}

impl Default for MarketOracleWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            oracle::contract::execute,
            oracle::contract::instantiate,
            oracle::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_oracle_query(deps: Deps, env: Env, msg: oracle::msg::QueryMsg) -> StdResult<Binary> {
    let res = match msg {
        oracle::msg::QueryMsg::PriceFor { denoms: _ } => to_binary(&oracle::msg::PriceResponse {
            prices: vec![DenomToPrice {
                denom: "unolus".to_string(),
                price: Price::new(Decimal::from_str("0.123456789").unwrap(), "UST".to_string()),
            }],
        }),
        _ => Ok(oracle::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}
