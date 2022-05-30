use cosmwasm_std::{
    coins, to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint64,
};
use cw_multi_test::{App, ContractWrapper, Executor};
use lpp::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use super::ADMIN;

pub struct MockLpp {
    contract_wrapper: Box<
        ContractWrapper<
            ExecuteMsg,
            InstantiateMsg,
            QueryMsg,
            ContractError,
            ContractError,
            ContractError,
        >,
    >,
}

impl MockLpp {
    pub fn with_contract_wrapper(
        contract: ContractWrapper<
            ExecuteMsg,
            InstantiateMsg,
            QueryMsg,
            ContractError,
            ContractError,
            ContractError,
        >,
    ) -> Self {
        Self {
            contract_wrapper: Box::new(contract),
        }
    }
    #[track_caller]
    pub fn instantiate(self, app: &mut App, lease_code_id: Uint64, denom: &str) -> (Addr, u64) {
        let lpp_id = app.store_code(self.contract_wrapper);
        let msg = lpp::msg::InstantiateMsg {
            denom: denom.to_string(),
            lease_code_id,
        };
        (
            app.instantiate_contract(
                lpp_id,
                Addr::unchecked(ADMIN),
                &msg,
                &coins(400, denom),
                "lpp",
                None,
            )
            .unwrap(),
            lpp_id,
        )
    }
}

impl Default for MockLpp {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            lpp::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_lpp_query(
    deps: Deps,
    env: Env,
    msg: lpp::msg::QueryMsg,
) -> Result<Binary, ContractError> {
    let res = match msg {
        lpp::msg::QueryMsg::LppBalance => to_binary(&lpp::msg::LppBalanceResponse {
            balance: Coin::new(1000000000, "UST"),
            total_principal_due: Coin::new(1000000000, "UST"),
            total_interest_due: Coin::new(1000000000, "UST"),
        }),
        _ => Ok(lpp::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}

pub fn mock_lpp_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: lpp::msg::ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        lpp::msg::ExecuteMsg::DistributeRewards => Ok(Response::default()),
        _ => Ok(lpp::contract::execute(deps, env, info, msg)?),
    }
}
