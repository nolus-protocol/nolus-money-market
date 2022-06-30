use std::collections::HashSet;

use cosmwasm_std::{
    from_binary, Addr, Binary, CosmosMsg, DepsMut, Response, StdResult, Storage, SubMsg, Timestamp,
};
use marketprice::{
    feed::{Denom, DenomToPrice},
    hooks::price::PriceHooks,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use time_oracle::{Alarms, Id, TimeOracle};

use crate::{
    msg::{ExecuteAlarmMsg, ExecuteHookMsg},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketAlarms {}

impl MarketAlarms {
    const TIME_ORACLE: TimeOracle<'static> = TimeOracle::new("time_oracle");
    const TIME_ALARMS: Alarms<'static> = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
    const PRICE_ALARMS: PriceHooks<'static> = PriceHooks::new("hooks");

    pub fn remove(storage: &mut dyn Storage, msg_id: Id) -> StdResult<()> {
        Self::TIME_ALARMS.remove(storage, msg_id)
    }

    pub fn try_add(deps: DepsMut, addr: Addr, time: Timestamp) -> Result<Response, ContractError> {
        let valid = deps
            .api
            .addr_validate(addr.as_str())
            .map_err(|_| ContractError::InvalidAlarmAddress(addr))?;
        Self::TIME_ALARMS.add(deps.storage, valid, time)?;
        Ok(Response::new().add_attribute("method", "try_add_alarm"))
    }

    pub fn try_notify(storage: &mut dyn Storage, ctime: Timestamp) -> StdResult<Response> {
        use time_oracle::AlarmDispatcher;

        struct OracleAlarmDispatcher<'a> {
            pub response: &'a mut Response,
        }

        impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
            fn send_to(&mut self, id: Id, addr: Addr, ctime: Timestamp) -> StdResult<()> {
                let msg = ExecuteAlarmMsg::Alarm(ctime);
                let wasm_msg = cosmwasm_std::wasm_execute(addr, &msg, vec![])?;
                let submsg = SubMsg::reply_always(CosmosMsg::Wasm(wasm_msg), id);
                self.response.messages.push(submsg);
                Ok(())
            }
        }

        let mut response = Response::new();
        let mut dispatcher = OracleAlarmDispatcher {
            response: &mut response,
        };

        Self::TIME_ALARMS.notify(storage, &mut dispatcher, ctime)?;

        Ok(response)
    }

    pub fn update_global_time(
        storage: &mut dyn Storage,
        block_time: Timestamp,
    ) -> StdResult<Response> {
        Self::TIME_ORACLE.update_global_time(storage, block_time)?;
        Self::try_notify(storage, block_time)
    }

    pub fn try_add_price_hook(
        storage: &mut dyn Storage,
        addr: Addr,
        target: DenomToPrice,
    ) -> Result<Response, ContractError> {
        // TODO: Check if sender address is a contract

        Self::PRICE_ALARMS.add_or_update(storage, &addr, target)?;
        Ok(Response::new().add_attribute("method", "try_add_price_hook"))
    }

    pub fn get_hook_denoms(storage: &dyn Storage) -> StdResult<HashSet<Denom>> {
        Self::PRICE_ALARMS.get_hook_denoms(storage)
    }
    pub fn try_notify_hooks(
        storage: &mut dyn Storage,
        updated_prices: Vec<DenomToPrice>,
    ) -> StdResult<()> {
        let messages: Vec<_> = Self::PRICE_ALARMS
            .get_affected(storage, updated_prices)?
            .iter()
            .map(|(addr, current_price)| Self::trigger_msg(addr, current_price))
            .collect();

        Ok(())
    }

    pub fn try_notify_hooks1(
        storage: &mut dyn Storage,
        receiver: &Addr,
        current_price: &DenomToPrice,
    ) -> StdResult<Response> {
        use time_oracle::AlarmDispatcher;

        struct OracleAlarmDispatcher<'a> {
            pub response: &'a mut Response,
        }

        impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
            fn send_to(&mut self, id: Id, addr: Addr, current_price: Binary) -> StdResult<()> {
                let msg = ExecuteHookMsg::Notify(from_binary(&current_price)?);
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

        Self::TIME_ALARMS.notify(storage, &mut dispatcher, ctime)?;

        Ok(response)
    }

    fn trigger_msg(receiver: &Addr, current: &DenomToPrice) -> StdResult<SubMsg> {
        let msg = ExecuteHookMsg::Notify(current.to_owned());
        let wasm_msg = cosmwasm_std::wasm_execute(receiver.to_string(), &msg, vec![])?;
        let submsg = SubMsg::reply_always(CosmosMsg::Wasm(wasm_msg), 1);
        Ok(submsg)
    }
}
