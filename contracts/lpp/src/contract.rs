#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Decimal,
    WasmQuery, QueryRequest, ContractInfoResponse, QuerierWrapper, BankMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QueryQuoteResponse, QueryBorrowResponse, QueryBorrowOutstandingInterestResponse};
use crate::state::{self, Config, State, Loan, CONFIG, STATE, LOANS, NANOSECS_IN_YEAR};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config::new(msg.denom, msg.loan_code_id);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

	let loan_unchecked = info.sender;

    match msg {
        ExecuteMsg::Borrow { amount } => try_borrow(deps, env, loan_unchecked, amount),
        ExecuteMsg::Repay { amount } => try_repay(loan_unchecked, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Quote { amount } => to_binary(&query_quote(&deps, &env, amount)?),
        QueryMsg::Borrow { loan } => to_binary(&query_borrow(loan)?),
        QueryMsg::BorrowOutstandingInterest {
            loan,
            outstanding_by,
        } => to_binary(&query_borrow_outstanding_interest(loan, outstanding_by)?),
    }?;

	Ok(res)
}

fn try_borrow(deps: DepsMut, env: Env, addr: Addr, amount: Coin) -> Result<Response, ContractError> {
	let config = CONFIG.load(deps.storage)?;
    let checked_loan_addr = validate_loan_addr(&deps.querier, &config, addr)?;
    let current_time = env.block.time;
    // let amount = validate_coins(&config, &amount)?;

    let annual_interest_rate = match query_quote(&deps.as_ref(), &env, amount.clone())? {
		QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
		QueryQuoteResponse::NoLiquidity => Err(ContractError::NoLiquidity {}),
    }?;

	if LOANS.has(deps.storage, checked_loan_addr.clone()) {
		return Err(ContractError::LoanExists {})
	}

	let loan = Loan {
    	principal_due: amount.amount,
    	annual_interest_rate,
    	interest_paid_by: current_time,
	};

	LOANS.save(deps.storage, checked_loan_addr.clone(), &loan)?;

	STATE.update(deps.storage, |mut state| -> Result<State, ContractError> {

    	let dt = Uint128::new((current_time.nanos() - state.last_update_time.nanos()).into());
        state.total_last_interest += state.total_principal_due * state.annual_interest_rate * dt / NANOSECS_IN_YEAR;
    	state.annual_interest_rate = Decimal::from_ratio(
        	state.annual_interest_rate*state.total_principal_due
        	+ loan.annual_interest_rate*loan.principal_due,
    	state.total_principal_due + loan.principal_due);

        state.total_principal_due+=amount.amount;
        state.last_update_time = current_time;

		Ok(state)
	})?;

	let transfer_msg = BankMsg::Send {
		to_address: checked_loan_addr.to_string(),
		amount: vec![amount],
	};

	let response = Response::new()
    	.add_attribute("method", "try_borrow")
    	.add_message(transfer_msg);

	Ok(response)
}

fn try_repay(_loan: Addr, _amount: Coin) -> Result<Response, ContractError> {
    unimplemented!()
}

fn query_quote(deps: &Deps, env: &Env, quote: Coin) -> Result<QueryQuoteResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
	let quote = validate_coins(&config, &quote)?;

	let balance = state::balance(&deps.querier, env, &config)?;

	if quote > balance.amount {
		return Ok(QueryQuoteResponse::NoLiquidity);
	}

	let State {
        total_principal_due,
        total_last_interest,
        annual_interest_rate,
        last_update_time,
	} = STATE.load(deps.storage)?;

	// NOTE: Should we be paranoid here and use checked calcs?
	let delta_t = Uint128::new((env.block.time.nanos() - last_update_time.nanos()).into());
	let total_interest = total_last_interest + total_principal_due*annual_interest_rate*delta_t/NANOSECS_IN_YEAR;
	let total_liability = total_principal_due + quote + total_interest;

	// NOTE: unused formula in the task
	let _utilization = total_liability/(total_liability+balance.amount);

	// NOTE: Do we need percent ratios? Maybe we can use relative values?
	let quote_interest_rate: Decimal = Decimal::from_ratio(100u128,1u128)*(config.base_interest_rate + config.utilization_optimal*config.addon_optimal_interest_rate);

	Ok(QueryQuoteResponse::QuoteInterestRate(quote_interest_rate))

}

fn query_borrow(_loan: Addr) -> Result<QueryBorrowResponse, ContractError> {
    unimplemented!()
}

fn query_borrow_outstanding_interest(
    _loan: Addr,
    _outstanding_by: Timestamp,
) -> Result<QueryBorrowOutstandingInterestResponse, ContractError> {
    unimplemented!()
}

fn validate_coins(config: &Config, coins: &Coin) -> Result<Uint128, ContractError> {
	if config.denom != coins.denom {
    	return Err(ContractError::Denom{
        	contract_denom: config.denom.clone(),
        	query_denom: coins.denom.clone(),
    	})
	}
	Ok(coins.amount)
}

fn validate_loan_addr(querier: &QuerierWrapper, config: &Config, addr: Addr) -> Result<Addr, ContractError> {
    let q_msg = QueryRequest::Wasm(WasmQuery::ContractInfo {contract_addr: addr.to_string()});
    let q_resp: ContractInfoResponse = querier.query(&q_msg)?;

    if q_resp.code_id != config.loan_code_id.u64() {
		return Err(ContractError::ContractId {})
    }

	Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, Uint64};
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "ust"));

        let msg = InstantiateMsg {
            denom: "ust".into(),
            loan_code_id: Uint64::new(1000),
        };
        let info = mock_info("creator", &coins(1000, "ust"));

        instantiate(deps.as_mut(), mock_env(), info, msg).expect("can't instantiate");
    }
}
