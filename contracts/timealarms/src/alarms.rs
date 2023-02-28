use serde::{Deserialize, Serialize};

use platform::contract;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env, StdResult, Storage, Timestamp},
    schemars::{self, JsonSchema},
};
use time_oracle::{Alarms, AlarmsCount, Id};

use crate::{dispatcher::OracleAlarmDispatcher, msg::AlarmsStatusResponse, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct TimeAlarms {}

impl TimeAlarms {
    const TIME_ALARMS: Alarms<'static> = Alarms::new("alarms", "alarms_idx", "alarms_next_id");

    pub fn remove(storage: &mut dyn Storage, msg_id: Id) -> StdResult<()> {
        Self::TIME_ALARMS.remove(storage, msg_id)
    }

    pub fn try_add(
        deps: DepsMut<'_>,
        env: Env,
        address: Addr,
        time: Timestamp,
    ) -> Result<Response, ContractError> {
        if time < env.block.time {
            return Err(ContractError::InvalidAlarm(time));
        }
        contract::validate_addr(&deps.querier, &address)?;
        Self::TIME_ALARMS.add(deps.storage, address, time)?;
        Ok(Response::new())
    }

    pub fn try_notify(
        storage: &mut dyn Storage,
        ctime: Timestamp,
        max_count: AlarmsCount,
    ) -> Result<Response, ContractError> {
        let dispatcher = OracleAlarmDispatcher::new();
        let dispatcher = Self::TIME_ALARMS.notify(storage, dispatcher, ctime, max_count)?;
        dispatcher.try_into()
    }

    pub fn try_any_alarm(
        storage: &dyn Storage,
        ctime: Timestamp,
    ) -> Result<AlarmsStatusResponse, ContractError> {
        Self::TIME_ALARMS
            .any_alarm(storage, ctime)
            .map(|remaining_alarms| AlarmsStatusResponse { remaining_alarms })
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use platform::contract;
    use sdk::cosmwasm_std::{
        testing::{self, mock_dependencies, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use crate::{alarms::TimeAlarms, ContractError};

    #[test]
    fn try_add_invalid_contract_address() {
        let mut deps = mock_dependencies();
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let msg_sender = Addr::unchecked("some address");
        assert!(TimeAlarms::try_add(
            deps.as_mut(),
            env.clone(),
            msg_sender.clone(),
            Timestamp::from_nanos(8)
        )
        .is_err());

        let expected_error: ContractError =
            contract::validate_addr(&deps.as_mut().querier, &msg_sender)
                .unwrap_err()
                .into();

        let result = TimeAlarms::try_add(deps.as_mut(), env, msg_sender, Timestamp::from_nanos(8))
            .unwrap_err();

        assert_eq!(expected_error, result);
    }

    #[test]
    fn try_add_valid_contract_address() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(contract::testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);
        let mut deps_temp = mock_dependencies();
        let mut deps = deps_temp.as_mut();
        deps.querier = querier;
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let msg_sender = Addr::unchecked("some address");
        assert!(TimeAlarms::try_add(deps, env, msg_sender, Timestamp::from_nanos(4)).is_ok());
    }

    #[test]
    fn try_add_alarm_in_the_past() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(contract::testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);
        let mut deps_temp = mock_dependencies();
        let mut deps = deps_temp.as_mut();
        deps.querier = querier;

        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_seconds(100);

        let msg_sender = Addr::unchecked("some address");
        TimeAlarms::try_add(deps, env, msg_sender, Timestamp::from_nanos(4)).unwrap_err();
    }
}
