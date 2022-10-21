use currency::lpn::Usdc;
use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, Uint64},
    cw_multi_test::Executor,
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
    pub fn instantiate<Lpn>(
        self,
        app: &mut MockApp,
        lease_code_id: Uint64,
        init_balance: Coin<Lpn>,
    ) -> (Addr, u64)
    where
        Lpn: Currency,
    {
        let lpp_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            lpn_ticker: Lpn::TICKER.into(),
            lease_code_id,
        };

        let funds = if init_balance.is_zero() {
            vec![]
        } else {
            vec![coin_legacy::to_cosmwasm(init_balance)]
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

pub fn mock_lpp_query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
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

pub fn mock_lpp_quote_query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
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
