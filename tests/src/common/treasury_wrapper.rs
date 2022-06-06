use cosmwasm_std::{Addr, StdError};
use cw_multi_test::ContractWrapper;

use cosmwasm_std::{coins, to_binary, Binary, Deps, Env, StdResult};
use cw_multi_test::{App, Executor};
use serde::{Deserialize, Serialize};
use treasury::ContractError;

use super::ADMIN;

pub fn treasury_instantiate_msg() -> treasury::msg::InstantiateMsg {
    treasury::msg::InstantiateMsg {}
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct QueryMsg {}

pub struct TreasuryWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            treasury::msg::ExecuteMsg,
            treasury::msg::InstantiateMsg,
            QueryMsg,
            ContractError,
            ContractError,
            StdError,
        >,
    >,
}

impl TreasuryWrapper {
    fn mock_treasury_query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
        to_binary(&MockResponse {})
    }
    #[track_caller]
    pub fn instantiate(self, app: &mut App, denom: &str) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
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
}

impl Default for TreasuryWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            treasury::contract::execute,
            treasury::contract::instantiate,
            Self::mock_treasury_query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
