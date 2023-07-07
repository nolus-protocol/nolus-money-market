use currency::{lpn::Usdc, Currency};
use finance::{coin::Coin, percent::Percent};
use lpp::{
    borrow::InterestRate,
    contract::sudo,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use sdk::{
    cosmwasm_std::{to_binary, Addr, Binary, Coin as CwCoin, Deps, Env, Uint64},
    cw_multi_test::AppResponse,
    testing::CwContract,
};

use crate::common::test_case::app::App;

use super::{CwContractWrapper, ADMIN};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate_default<Lpn>(
        app: &mut App,
        lease_code_id: Uint64,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> (Addr, u64)
    where
        Lpn: Currency,
    {
        let endpoints: CwContractWrapper<_, _, _, _, _, _, _, _, _, _, _> = CwContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            lpp::contract::query,
        )
        .with_sudo(sudo);

        Self::instantiate::<Lpn>(
            app,
            Box::new(endpoints),
            lease_code_id,
            init_balance,
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    #[track_caller]
    pub fn instantiate<Lpn>(
        app: &mut App,
        endpoints: Box<CwContract>,
        lease_code_id: Uint64,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> (Addr, u64)
    where
        Lpn: Currency,
    {
        let lpp_id = app.store_code(endpoints);
        let lease_code_admin = Addr::unchecked("contract5");
        let msg = InstantiateMsg {
            lpn_ticker: Lpn::TICKER.into(),
            lease_code_admin: lease_code_admin.clone(),
            borrow_rate: InterestRate::new(
                base_interest_rate,
                utilization_optimal,
                addon_optimal_interest_rate,
            )
            .expect("Couldn't construct interest rate value!"),
        };

        let lpp = app
            .instantiate(
                lpp_id,
                Addr::unchecked(ADMIN),
                &msg,
                init_balance,
                "lpp",
                None,
            )
            .unwrap()
            .unwrap_response();
        let _: AppResponse = app
            .execute(
                lease_code_admin,
                lpp.clone(),
                &ExecuteMsg::NewLeaseCode { lease_code_id },
                &[],
            )
            .unwrap()
            .unwrap_response();

        (lpp, lpp_id)
    }
}

pub(crate) fn mock_lpp_query(
    deps: Deps<'_>,
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

pub(crate) fn mock_lpp_quote_query(
    deps: Deps<'_>,
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
