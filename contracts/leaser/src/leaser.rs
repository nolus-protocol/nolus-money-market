use std::collections::HashSet;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, WasmMsg,
};

use finance::coin_legacy::{from_cosmwasm, to_cosmwasm};
use finance::currency::Usdc;
use finance::liability::Liability;
use finance::percent::Percent;
use lease::msg::{LoanForm, NewLeaseForm};

use crate::error::ContractError;
use crate::lpp_querier::LppQuerier;
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
                    msg: to_binary(&Leaser::open_lease_msg(sender, config, currency))?,
                }),
                instance_reply_id,
            )]),
        )
    }

    pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
        let config = Config::load(deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn query_loans(deps: Deps, owner: Addr) -> StdResult<HashSet<Addr>> {
        Loans::get(deps.storage, owner)
    }

    pub fn query_quote(
        _env: Env,
        deps: Deps,
        downpayment: cosmwasm_std::Coin,
    ) -> StdResult<QuoteResponse> {
        // borrowUST = LeaseInitialLiability% * downpaymentUST / (1 - LeaseInitialLiability%)
        if downpayment.amount.is_zero() {
            return Err(StdError::generic_err(
                "cannot open lease with zero downpayment",
            ));
        }
        let dp = from_cosmwasm::<Usdc>(downpayment.clone())
            .map_err(|err| StdError::generic_err(err.to_string()))?;

        let config = Config::load(deps.storage)?;

        let borrow_amount = config.liability.init_borrow_amount(dp);
        let total_amount = borrow_amount + dp;

        let annual_interest_rate = LppQuerier::get_annual_interest_rate(deps, downpayment)?;

        Ok(QuoteResponse {
            total: to_cosmwasm(total_amount),
            borrow: to_cosmwasm(borrow_amount),
            annual_interest_rate: annual_interest_rate + config.lease_interest_rate_margin,
        })
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
                lpp: config.lpp_ust_addr.into_string(),
                interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
                grace_period_secs: config.repayment.grace_period_sec,
            },
        }
    }
}
