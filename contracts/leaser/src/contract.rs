#[cfg(feature = "cosmwasm_bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::helpers::open_lease_msg;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, Liability, QueryMsg, QuoteResponse, Repayment,
};
use crate::state::LS;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm_bindings", entry_point)]
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

#[cfg_attr(feature = "cosmwasm_bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Borrow {} => try_borrow(deps, info.funds, info.sender),
        ExecuteMsg::Config {
            lease_interest_rate_margin,
            liability,
            repayment,
        } => try_configure(deps, info, lease_interest_rate_margin, liability, repayment),
    }
}

#[cfg_attr(feature = "cosmwasm_bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&query_quote(env, deps, downpayment)?),
    }
}

#[cfg_attr(feature = "cosmwasm_bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr_raw = parse_reply_instantiate_data(msg.clone())
        .map(|r| r.contract_address)
        .map_err(|_| ContractError::ParseError {})?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;
    register_lease(deps, msg.id, contract_addr)
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    lease_interest_rate_margin: u8,
    liability: Liability,
    repayment: Repayment,
) -> Result<Response, ContractError> {
    let config = LS.get_config(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    LS.update_config(
        deps.storage,
        lease_interest_rate_margin,
        liability,
        repayment,
    )?;

    Ok(Response::default())
}

pub fn try_borrow(
    deps: DepsMut,
    amount: Vec<Coin>,
    sender: Addr,
) -> Result<Response, ContractError> {
    let config = LS.get_config(deps.storage)?;
    let instance_reply_id = LS.next(deps.storage, sender.clone())?;
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: config.lease_code_id,
                funds: amount,
                label: "lease".to_string(),
                msg: to_binary(&open_lease_msg(sender, config))?,
            }),
            instance_reply_id,
        )]),
    )
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
    let numerator = Uint128::from(config.liability.initial) * downpayment.amount;
    let denominator = Uint128::from(100 - config.liability.initial);

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

fn register_lease(deps: DepsMut, msg_id: u64, lease_addr: Addr) -> Result<Response, ContractError> {
    // TODO: Remove pending id if the creation was not successful
    LS.save(deps.storage, msg_id, lease_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", lease_addr))
}

#[cfg(test)]
fn get_annual_interest_rate(_deps: Deps, _downpayment: Coin) -> StdResult<Decimal> {
    Ok(Decimal::one())
}
