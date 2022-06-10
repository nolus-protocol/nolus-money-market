use cosmwasm_std::{Coin, Deps, StdResult};
use finance::percent::Percent;

pub struct LppQuerier {}
impl LppQuerier {
    #[cfg(not(test))]
    pub fn get_annual_interest_rate(deps: Deps, downpayment: Coin) -> StdResult<Percent> {
        use cosmwasm_std::StdError;
        use lpp::msg::{QueryMsg as LppQueryMsg, QueryQuoteResponse};

        use crate::state::config::Config;

        let config = Config::load(deps.storage)?;
        let query_msg: LppQueryMsg = LppQueryMsg::Quote {
            amount: downpayment,
        };
        let query_response: QueryQuoteResponse = deps
            .querier
            .query_wasm_smart(config.lpp_ust_addr.to_string(), &query_msg)?;
        match query_response {
            QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
            QueryQuoteResponse::NoLiquidity => Err(StdError::generic_err("NoLiquidity")),
        }
    }

    #[cfg(test)]
    pub fn get_annual_interest_rate(_deps: Deps, _downpayment: Coin) -> StdResult<Percent> {
        Ok(Percent::HUNDRED)
    }
}
