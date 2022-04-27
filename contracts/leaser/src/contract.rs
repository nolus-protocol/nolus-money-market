use std::ops::{Mul, Sub};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use lease::msg::InstantiateMsg as LeaseInstantiateMsg;

use crate::error::ContractError;
use crate::helpers::assert_sent_sufficient_coin;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, QuoteResponse};
use crate::state::{Config, CONFIG, INSTANTIATE_REPLY_IDS, LEASES, PENDING_INSTANCE_CREATIONS};

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
    let config = Config::new(info.sender, msg)?;
    CONFIG.save(deps.storage, &config)?;

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
    let config = CONFIG.load(deps.storage)?;
    assert_sent_sufficient_coin(&amount, config.lease_minimal_downpayment)?;

    let instance_reply_id = INSTANTIATE_REPLY_IDS.next(deps.storage)?;
    PENDING_INSTANCE_CREATIONS.save(deps.storage, instance_reply_id, &sender)?;
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: config.lease_code_id,
                funds: amount,
                label: "lease".to_string(),
                msg: to_binary(&LeaseInstantiateMsg {
                    owner: sender.to_string(),
                })?,
            }),
            instance_reply_id,
        )]),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&query_quote(deps, downpayment)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_quote(deps: Deps, downpayment_ust: Uint128) -> StdResult<QuoteResponse> {
    // borrowUST = LeaseInitialLiability% * downpaymentUST / (1 - LeaseInitialLiability%)
    if downpayment_ust.is_zero() {
        return Err(StdError::generic_err(
            "cannot open lease with zero downpayment",
        ));
    }
    let config = CONFIG.load(deps.storage)?;

    // TODO: too complex, maybe can be represented in more rust native way
    let numerator = config
        .lease_initial_liability
        .mul(Decimal::from_atomics(downpayment_ust, 0).unwrap());
    let denominator = Decimal::one().sub(config.lease_initial_liability);
    let borrow_ust = numerator.mul(denominator.denominator()) / denominator.numerator();

    Ok(QuoteResponse {
        total_ust: borrow_ust + downpayment_ust,
        borrow_ust,
        annual_interest_rate: Decimal::one(), // hardcoded until LPP contract is merged
    })
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
    let owner_addr = PENDING_INSTANCE_CREATIONS.load(deps.storage, msg_id)?;
    LEASES.save(deps.storage, &owner_addr, &lease_addr)?;
    PENDING_INSTANCE_CREATIONS.remove(deps.storage, msg_id);
    Ok(Response::new().add_attribute("lease_address", lease_addr))
}
