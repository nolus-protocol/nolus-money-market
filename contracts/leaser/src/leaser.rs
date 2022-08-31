use std::collections::HashSet;

use cosmwasm_std::{Addr, Deps, DepsMut, MessageInfo, Response, StdResult};

use finance::{coin::CoinDTO, liability::Liability, percent::Percent};
use lpp::stub::LppRef;

use crate::{
    cmd::Quote,
    error::{ContractError, ContractResult},
    msg::{ConfigResponse, QuoteResponse, Repayment},
    state::config::Config,
    state::leaser::Loans,
};

pub struct Leaser {}

impl Leaser {
    pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
        let config = Config::load(deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn query_loans(deps: Deps, owner: Addr) -> StdResult<HashSet<Addr>> {
        Loans::get(deps.storage, owner)
    }

    pub fn query_quote(deps: Deps, downpayment: CoinDTO) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(deps.storage)?;

        let lpp = LppRef::try_from(
            config.lpp_addr.to_string(),
            deps.api,
            &deps.querier,
            lease::constants::ReplyId::OpenLoanReq as u64,
        )?;

        let resp = lpp.execute(
            Quote::new(
                downpayment,
                config.liability,
                config.lease_interest_rate_margin,
            )?,
            &deps.querier,
        )?;

        Ok(resp)
    }

    pub fn try_configure(
        deps: DepsMut,
        info: MessageInfo,
        lease_interest_rate_margin: Percent,
        liability: Liability,
        repayment: Repayment,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        if info.sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        if liability.invariant_held().is_err() {
            return Err(ContractError::IvalidLiability {});
        }
        repayment.validate_period()?;
        Config::update(
            deps.storage,
            lease_interest_rate_margin,
            liability,
            repayment,
        )?;

        Ok(Response::default())
    }
}
