use std::collections::HashSet;

use cosmwasm_std::{
    from_binary, Addr, Binary, CosmosMsg, Response, StdError, StdResult, Storage, SubMsg, Timestamp,
};
use marketprice::{
    alarms::{price::PriceHooks, AlarmDispatcher},
    feed::{Denom, DenomToPrice},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use time_oracle::Id;

use crate::{msg::ExecuteAlarmMsg, state::config::Config, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketAlarms {}

impl MarketAlarms {
    const PRICE_ALARMS: PriceHooks<'static> = PriceHooks::new("hooks", "hooks_sequence");

    pub fn remove(storage: &mut dyn Storage, addr: Addr) -> Result<Response, ContractError> {
        Ok(Self::PRICE_ALARMS.remove(storage, addr)?)
    }

    pub fn try_add_price_alarm(
        storage: &mut dyn Storage,
        addr: Addr,
        target: DenomToPrice,
    ) -> Result<Response, ContractError> {
        Self::PRICE_ALARMS.add_or_update(storage, &addr, target)?;
        Ok(Response::new().add_attribute("method", "try_add_price_hook"))
    }

    pub fn get_hook_denoms(storage: &dyn Storage) -> StdResult<HashSet<Denom>> {
        Self::PRICE_ALARMS.get_hook_denoms(storage)
    }

    pub fn try_notify_hooks(
        storage: &mut dyn Storage,
        ctime: Timestamp,
        updated_prices: Vec<DenomToPrice>,
    ) -> StdResult<Response> {
        struct OracleAlarmDispatcher<'a> {
            pub response: &'a mut Response,
        }

        impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
            fn send_to(
                &mut self,
                id: Id,
                addr: Addr,
                _ctime: Timestamp,
                data: &Option<Binary>,
            ) -> StdResult<()> {
                let current_price: DenomToPrice = match data {
                    Some(bin) => from_binary(bin)?,
                    None => return Err(StdError::generic_err("msg")),
                };

                let msg = ExecuteAlarmMsg::Alarm(current_price);
                let wasm_msg = cosmwasm_std::wasm_execute(addr.to_string(), &msg, vec![])?;
                let submsg = SubMsg::reply_always(CosmosMsg::Wasm(wasm_msg), id);
                self.response.messages.push(submsg);
                Ok(())
            }
        }

        let mut response = Response::new();
        let mut dispatcher = OracleAlarmDispatcher {
            response: &mut response,
        };

        Self::PRICE_ALARMS.notify(storage, &mut dispatcher, ctime, updated_prices)?;

        Ok(response)
    }

    pub fn trigger_time_alarms(storage: &mut dyn Storage) -> StdResult<SubMsg> {
        let config = Config::load(storage)?;

        let msg = timealarms::msg::ExecuteMsg::Notify();
        let wasm_msg = cosmwasm_std::wasm_execute(config.timealarms_contract, &msg, vec![])?;
        Ok(SubMsg::reply_on_error(CosmosMsg::Wasm(wasm_msg), 1))
    }
}
