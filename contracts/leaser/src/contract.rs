#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use lease::msg::InstantiateMsg as LoanInstantiateMsg;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, INSTANTIATE_REPLY_IDS, LOANS, PENDING_INSTANCE_CREATIONS};

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
        ExecuteMsg::Borrow {} => try_borrow(deps, info),
    }
}

pub fn try_borrow(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let instance_reply_id = INSTANTIATE_REPLY_IDS.next(deps.storage)?;
    PENDING_INSTANCE_CREATIONS.save(deps.storage, instance_reply_id, &info.sender)?;
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: config.lease_code_id,
                funds: vec![],
                label: "lease".to_string(),
                msg: to_binary(&LoanInstantiateMsg {
                    owner: info.sender.to_string(),
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
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr_raw = parse_reply_instantiate_data(msg.clone())
        .map(|r| r.contract_address)
        .map_err(|_| ContractError::ParseError {})?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;
    register_loan(deps, msg.id, contract_addr)
}

fn register_loan(deps: DepsMut, msg_id: u64, loan_addr: Addr) -> Result<Response, ContractError> {
    // TODO: Remove pending id if the creation was not successful
    let owner_addr = PENDING_INSTANCE_CREATIONS.load(deps.storage, msg_id)?;
    LOANS.save(deps.storage, &owner_addr, &loan_addr)?;
    PENDING_INSTANCE_CREATIONS.remove(deps.storage, msg_id);
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Uint256};

    use super::*;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            lease_code_id: 1,
            lpp_ust_addr: Addr::unchecked("test"),
            lease_interest_rate_margin: 3,
            lease_max_liability: 80,
            lease_healthy_liability: 70,
            lease_initial_liability: 65,
            repayment_period_nano_sec: Uint256::from(123_u64),
            grace_period_nano_sec: Uint256::from(123_u64),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config_response: ConfigResponse = from_binary(&res).unwrap();
        let config = config_response.config;
        assert_eq!("creator", config.owner);
        assert_eq!(1, config.lease_code_id);
        assert_eq!(Addr::unchecked("test"), config.lpp_ust_addr);
    }
}
