use cosmwasm_std::{BankMsg, DepsMut, Env, MessageInfo, Response, StdResult, Storage};

use crate::{
    msg::ConfigResponse,
    state::{config::Config, transfer_log::TransferLog},
    ContractError,
};

pub struct Profit {}

impl Profit {
    pub fn try_config(
        deps: DepsMut,
        info: MessageInfo,
        cadence_hours: u32,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        if info.sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        Config::update(deps.storage, cadence_hours)?;

        Ok(Response::new().add_attribute("method", "config"))
    }
    pub fn transfer(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;

        if info.sender != config.time_oracle {
            return Err(ContractError::UnrecognisedAlarm(info.sender));
        }

        let balance = deps.querier.query_all_balances(env.contract.address)?;

        if balance.is_empty() {
            return Ok(Response::new()
                .add_attribute("method", "try_transfer")
                .add_attribute("result", "no profit to dispatch"));
        }

        TransferLog::update(deps.storage, env.block.time, &balance)?;
        Ok(Response::new()
            .add_attribute("method", "try_transfer")
            .add_message(BankMsg::Send {
                to_address: config.treasury.to_string(),
                amount: balance,
            }))
    }
    pub fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
        let config = Config::load(storage)?;
        Ok(ConfigResponse {
            cadence_hours: config.cadence_hours,
        })
    }
}
