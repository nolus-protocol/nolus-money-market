use cosmwasm_std::{Deps, StdResult};
use finance::coin::Coin;
use finance::currency::Currency;
use finance::percent::Percent;

// TODO use lpp::stub::Lpp<Lpn> instead of LppQuerier
pub struct LppQuerier {}
impl LppQuerier {
    #[cfg(not(test))]
    pub fn get_annual_interest_rate<Lpn>(deps: Deps, downpayment: Coin<Lpn>) -> StdResult<Percent>
    where
        Lpn: Currency,
    {
        use cosmwasm_std::StdError;

        use lpp::msg::{QueryMsg as LppQueryMsg, QueryQuoteResponse};

        use crate::state::config::Config;

        let config = Config::load(deps.storage)?;
        let query_msg: LppQueryMsg = LppQueryMsg::Quote {
            amount: downpayment.into(),
        };
        let query_response: QueryQuoteResponse = deps
            .querier
            .query_wasm_smart(config.lpp_addr.to_string(), &query_msg)?;
        match query_response {
            QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
            QueryQuoteResponse::NoLiquidity => Err(StdError::generic_err("NoLiquidity")),
        }
    }

    // TODO use a mock of lpp::stub::Lpp<Lpn> instead of this condiionally compilated function
    #[cfg(test)]
    pub fn get_annual_interest_rate<Lpn>(_deps: Deps, _downpayment: Coin<Lpn>) -> StdResult<Percent>
    where
        Lpn: Currency,
    {
        Ok(Percent::HUNDRED)
    }
}
