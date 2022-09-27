use cosmwasm_std::{Addr, DepsMut, Response, StdResult, Storage, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use currency::native::Nls;
use platform::batch::Batch;
use time_oracle::{AlarmError, Alarms, Id};

use crate::{contract_validation::validate_contract_addr, msg::ExecuteAlarmMsg, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct TimeAlarms {}

impl TimeAlarms {
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
        Ok(Response::new())
    }

    pub fn try_notify(
        storage: &mut dyn Storage,
        ctime: Timestamp,
    ) -> Result<Response, ContractError> {
        use time_oracle::AlarmDispatcher;

        struct OracleAlarmDispatcher<'a> {
            pub batch: &'a mut Batch,
        }

        impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
            fn send_to(&mut self, id: Id, addr: Addr, ctime: Timestamp) -> Result<(), AlarmError> {
                Ok(self.batch.schedule_execute_wasm_reply_always::<_, Nls>(
                    &addr,
                    ExecuteAlarmMsg::TimeAlarm(ctime),
                    None,
                    id,
                )?)
            }
        }

        let mut batch = Batch::default();
        let mut dispatcher = OracleAlarmDispatcher { batch: &mut batch };

        Self::TIME_ALARMS.notify(storage, &mut dispatcher, ctime)?;

        Ok(batch.into())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use crate::{
        alarms::TimeAlarms,
        contract_validation::{tests::valid_contract_query, validate_contract_addr},
        ContractError,
    };

    #[test]
    fn try_add_invalid_contract_address() {
        let mut deps = mock_dependencies();
        let msg_sender = Addr::unchecked("some address");
        assert!(
            TimeAlarms::try_add(deps.as_mut(), msg_sender.clone(), Timestamp::from_nanos(8))
                .is_err()
        );

        let expected_error = ContractError::Std(
            validate_contract_addr(&deps.as_mut().querier, &msg_sender).unwrap_err(),
        );

        let result =
            TimeAlarms::try_add(deps.as_mut(), msg_sender, Timestamp::from_nanos(8)).unwrap_err();

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
        assert!(TimeAlarms::try_add(deps, msg_sender, Timestamp::from_nanos(4)).is_ok());
    }
}
