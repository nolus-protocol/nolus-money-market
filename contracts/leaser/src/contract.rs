use std::ops::Sub;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use lease::liability::Liability;
use lease::opening::{LoanForm, NewLeaseForm};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, QuoteResponse};
use crate::state::LS;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    LS.init(deps, msg, info.sender)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Borrow {} => try_borrow(deps, info.funds, info.sender),
    }
}

pub fn try_borrow(
    deps: DepsMut,
    amount: Vec<Coin>,
    sender: Addr,
) -> Result<Response, ContractError> {
    let config = LS.get_config(deps.storage)?;
    // assert_sent_sufficient_coin(&amount, config.lease_minimal_downpayment)?;

    let instance_reply_id = LS.next(deps.storage, sender.clone())?;
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: config.lease_code_id,
                funds: amount,
                label: "lease".to_string(),
                msg: to_binary(&NewLeaseForm {
                    customer: sender.into_string(),
                    currency: "".to_owned(), // TODO the same denom lppUST is working with
                    liability: Liability::new(65, 5, 10, 20 * 24),
                    loan: LoanForm {
                        annual_margin_interest_permille: 31, // 3.1%
                        lpp: config.lpp_ust_addr.into_string(),
                        interest_due_period_secs: 90 * 24 * 60 * 60, // 90 days TODO use a crate for daytime calculations
                        grace_period_secs: 10 * 24 * 60 * 60, // 10 days TODO use a crate for daytime calculations
                    },
                })?,
            }),
            instance_reply_id,
        )]),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&query_quote(env, deps, downpayment)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = LS.get_config(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_quote(_env: Env, deps: Deps, downpayment: Coin) -> StdResult<QuoteResponse> {
    // borrowUST = LeaseInitialLiability% * downpaymentUST / (1 - LeaseInitialLiability%)
    if downpayment.amount.is_zero() {
        return Err(StdError::generic_err(
            "cannot open lease with zero downpayment",
        ));
    }
    let config = LS.get_config(deps.storage)?;
    let numerator = config.lease_initial_liability.numerator() * downpayment.amount;
    let denominator = Decimal::one()
        .sub(config.lease_initial_liability)
        .numerator();

    let borrow_amount = numerator / denominator;
    let total_amount = borrow_amount + downpayment.amount;

    Ok(QuoteResponse {
        total: Coin::new(total_amount.u128(), downpayment.denom.clone()),
        borrow: Coin::new(borrow_amount.u128(), downpayment.denom.clone()),
        annual_interest_rate: get_annual_interest_rate(deps, downpayment)?,
    })
}

#[cfg(not(test))]
fn get_annual_interest_rate(deps: Deps, downpayment: Coin) -> StdResult<Decimal> {
    use cosmwasm_std::{QueryRequest, WasmQuery};

    use crate::msg::{LPPQueryMsg, QueryQuoteResponse};

    let config = LS.get_config(deps.storage)?;
    let query_msg: LPPQueryMsg = LPPQueryMsg::Quote {
        amount: downpayment,
    };
    let query_response: QueryQuoteResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.lpp_ust_addr.to_string(),
            msg: to_binary(&query_msg)?,
        }))?;
    match query_response {
        QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
        QueryQuoteResponse::NoLiquidity => Err(StdError::generic_err("NoLiquidity")),
    }
}

#[cfg(test)]
fn get_annual_interest_rate(_deps: Deps, _downpayment: Coin) -> StdResult<Decimal> {
    Ok(Decimal::one())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr_raw = parse_reply_instantiate_data(msg.clone())
        .map(|r| r.contract_address)
        .map_err(|_| ContractError::ParseError {})?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;
    register_lease(deps, msg.id, contract_addr)
}

fn register_lease(deps: DepsMut, msg_id: u64, lease_addr: Addr) -> Result<Response, ContractError> {
    // TODO: Remove pending id if the creation was not successful
    LS.save(deps.storage, msg_id, lease_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", lease_addr))
}
