#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // send collateral with the request?
        ExecuteMsg::RequestLoan { laon } => execute_transfer(deps, env, info, laon),
        // repay part of the loan
        ExecuteMsg::RepayLoan { laon } => execute_transfer(deps, env, info), 
        // pay all the fees + the loan
        ExecuteMsg:: CloseLoan {} => execute_execute(deps, env, info),
    }
}
