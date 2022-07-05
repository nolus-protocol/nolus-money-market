use cosmwasm_std::{
    Addr, ContractInfoResponse, CosmosMsg, DepsMut, Empty, QuerierWrapper, QueryRequest, Response,
    StdResult, Storage, SubMsg, Timestamp, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use time_oracle::{Alarms, Id, TimeOracle};

use crate::{msg::ExecuteAlarmMsg, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketAlarms {}

impl MarketAlarms {
    const TIME_ORACLE: TimeOracle<'static> = TimeOracle::new("time_oracle");
    const TIME_ALARMS: Alarms<'static> = Alarms::new("alarms", "alarms_idx", "alarms_next_id");

    pub fn remove(storage: &mut dyn Storage, msg_id: Id) -> StdResult<()> {
        Self::TIME_ALARMS.remove(storage, msg_id)
    }

    pub fn try_add(
        deps: DepsMut,
        address: Addr,
        time: Timestamp,
    ) -> Result<Response, ContractError> {
        validate_contract_addr(&deps.querier, address.clone())?;
        Self::TIME_ALARMS.add(deps.storage, address, time)?;
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
}

fn validate_contract_addr(querier: &QuerierWrapper, addr: Addr) -> StdResult<()> {
    let raw = QueryRequest::<Empty>::Wasm(WasmQuery::ContractInfo {
        contract_addr: addr.into_string(),
    });
    let res: StdResult<ContractInfoResponse> = querier.query(&raw);
    res.map(|_| ())
}
