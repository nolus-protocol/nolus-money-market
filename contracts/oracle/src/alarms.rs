use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Response, StdResult, Storage, SubMsg, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use time_oracle::{Alarms, Id, TimeOracle};

use crate::{contract_validation::validate_contract_addr, msg::ExecuteAlarmMsg, ContractError};

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
        validate_contract_addr(&deps.querier, &address)?;
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use super::MarketAlarms;
    use crate::contract_validation::{tests::valid_contract_query, validate_contract_addr};
    use crate::ContractError;

    #[test]
    fn try_add_invalid_contract_address() {
        let mut deps = mock_dependencies();
        let msg_sender = Addr::unchecked("some address");
        assert!(
            MarketAlarms::try_add(deps.as_mut(), msg_sender.clone(), Timestamp::from_nanos(8))
                .is_err()
        );

        let expected_error = ContractError::Std(
            validate_contract_addr(&deps.as_mut().querier, &msg_sender).unwrap_err(),
        );

        let result =
            MarketAlarms::try_add(deps.as_mut(), msg_sender, Timestamp::from_nanos(8)).unwrap_err();

        assert_eq!(expected_error, result);
    }

    #[test]
    fn try_add_valid_contract_address() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(valid_contract_query);
        let querier = QuerierWrapper::new(&mock_querier);
        let mut deps_temp = mock_dependencies();
        let mut deps = deps_temp.as_mut();
        deps.querier = querier;

        let msg_sender = Addr::unchecked("some address");
        assert!(MarketAlarms::try_add(deps, msg_sender, Timestamp::from_nanos(4)).is_ok());
    }
}
