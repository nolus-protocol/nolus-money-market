use std::str::FromStr;

use cosmwasm_std::{to_binary, Addr, Binary, Decimal, Deps, Empty, Env, StdResult};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use marketprice::feed::{DenomToPrice, Price};

use super::ADMIN;

fn mock_oracle_price() -> StdResult<oracle::msg::PriceResponse> {
    Ok(oracle::msg::PriceResponse {
        prices: vec![DenomToPrice {
            denom: "UST".to_string(),
            price: Price::new(Decimal::from_str("1000").unwrap(), "unolus".to_string()),
        }],
    })
}

// TODO: remove when lpp implements LppBalance
fn mock_oracle_query(deps: Deps, env: Env, msg: oracle::msg::QueryMsg) -> StdResult<Binary> {
    let res = match msg {
        oracle::msg::QueryMsg::PriceFor { denoms: _ } => to_binary(&mock_oracle_price()?),
        _ => Ok(oracle::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}

pub fn contract_oracle_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        oracle::contract::execute,
        oracle::contract::instantiate,
        mock_oracle_query,
    );
    Box::new(contract)
}

#[track_caller]
pub fn instantiate_oracle(app: &mut App, denom: &str) -> Addr {
    let code_id = app.store_code(contract_oracle_mock());
    let msg = oracle_instantiate_msg(denom.to_string());
    app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "oracle", None)
        .unwrap()
}

pub fn oracle_instantiate_msg(base_asset: String) -> oracle::msg::InstantiateMsg {
    oracle::msg::InstantiateMsg {
        base_asset,
        price_feed_period: 60,
        feeders_percentage_needed: 1,
        supported_denom_pairs: vec![("UST".to_string(), "unolus".to_string())],
    }
}
