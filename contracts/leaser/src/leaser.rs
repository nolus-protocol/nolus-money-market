use std::collections::HashSet;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, DepsMut, MessageInfo, Response, StdResult, SubMsg, WasmMsg,
};

use finance::coin::CoinDTO;
use finance::liability::Liability;
use finance::percent::Percent;
use lease::msg::{LoanForm, NewLeaseForm};
use lpp::stub::LppRef;

use crate::cmd::Quote;
use crate::error::{ContractError, ContractResult};
use crate::msg::{ConfigResponse, QuoteResponse, Repayment};
use crate::state::config::Config;
use crate::state::leaser::Loans;

pub struct Leaser {}

impl Leaser {
    pub fn try_borrow(
        deps: DepsMut,
        amount: Vec<cosmwasm_std::Coin>,
        sender: Addr,
        currency: String,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        let instance_reply_id = Loans::next(deps.storage, sender.clone())?;

        Ok(
            Response::new().add_submessages(vec![SubMsg::reply_on_success(
                CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id: config.lease_code_id,
                    funds: amount,
                    label: "lease".to_string(),
                    msg: to_binary(&Self::open_lease_msg(sender, config, currency))?,
                }),
                instance_reply_id,
            )]),
        )
    }

    pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
        let config = Config::load(deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn query_loans(deps: Deps, owner: Addr) -> StdResult<HashSet<Addr>> {
        Loans::get(deps.storage, owner)
    }

    pub fn query_quote(deps: Deps, downpayment: CoinDTO) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(deps.storage)?;

        let lpp = LppRef::try_from(config.lpp_addr.to_string(), deps.api, &deps.querier)?;

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
        Config::update(
            deps.storage,
            lease_interest_rate_margin,
            liability,
            repayment,
        )?;

        Ok(Response::default())
    }

    pub(crate) fn open_lease_msg(sender: Addr, config: Config, currency: String) -> NewLeaseForm {
        NewLeaseForm {
            customer: sender.into_string(),
            currency,
            liability: config.liability,
            loan: LoanForm {
                annual_margin_interest: config.lease_interest_rate_margin,
                lpp: config.lpp_addr.into_string(),
                interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
                grace_period_secs: config.repayment.grace_period_sec,
            },
        }
    }
}
