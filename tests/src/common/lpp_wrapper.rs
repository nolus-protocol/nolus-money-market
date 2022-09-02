use cosmwasm_std::{Addr, Binary, coins, Deps, Env, to_binary, Uint64};
use cw_multi_test::Executor;

use finance::{
    coin::Coin,
    currency::Usdc,
    percent::Percent
};
use lpp::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

pub struct LppWrapper {
    contract_wrapper: Box<LppContractWrapper>,
}

impl LppWrapper {
    pub fn with_contract_wrapper(
        contract: ContractWrapper<
            ExecuteMsg,
            ContractError,
            InstantiateMsg,
            ContractError,
            QueryMsg,
            ContractError,
        >,
    ) -> Self {
        Self {
            contract_wrapper: Box::new(contract),
        }
    }
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        lease_code_id: Uint64,
        denom: &str,
        balance: u128,
    ) -> (Addr, u64) {
        let lpp_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            denom: denom.to_string(),
            lease_code_id,
        };

        let funds = if balance == 0 {
            vec![]
        } else {
            coins(balance, denom)
        };

        (
            app.instantiate_contract(lpp_id, Addr::unchecked(ADMIN), &msg, &funds, "lpp", None)
                .unwrap(),
            lpp_id,
        )
    }
}

impl Default for LppWrapper {
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
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::LppBalance() => to_binary(&lpp::msg::LppBalanceResponse::<Usdc> {
            balance: Coin::new(1000000000),
            total_principal_due: Coin::new(1000000000),
            total_interest_due: Coin::new(1000000000),
            balance_nlpn: Coin::new(1000000000),
        }),
        _ => Ok(lpp::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}

pub fn mock_lpp_quote_query(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Quote { amount: _amount } => to_binary(
            &lpp::msg::QueryQuoteResponse::QuoteInterestRate(Percent::HUNDRED),
        ),
        _ => Ok(lpp::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}

type LppContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
>;
