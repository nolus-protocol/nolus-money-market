use cosmwasm_std::Addr;
use cw_multi_test::ContractWrapper;

use cosmwasm_std::{coins, to_binary, Binary, Deps, Empty, Env, StdResult};
use cw_multi_test::{App, Contract, Executor};
use serde::{Deserialize, Serialize};

use super::ADMIN;

pub fn treasury_instantiate_msg() -> treasury::msg::InstantiateMsg {
    treasury::msg::InstantiateMsg {}
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct QueryMsg {}

fn mock_treasury_query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    to_binary(&MockResponse {})
}

pub fn contract_treasury_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        treasury::contract::execute,
        treasury::contract::instantiate,
        mock_treasury_query,
    );
    Box::new(contract)
}

#[track_caller]
pub fn instantiate_treasury(app: &mut App, denom: &str) -> Addr {
    let code_id = app.store_code(contract_treasury_mock());
    let msg = treasury_instantiate_msg();

    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &coins(1000, denom),
        "treasury",
        None,
    )
    .unwrap()
}
