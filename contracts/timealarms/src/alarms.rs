use serde::{Deserialize, Serialize};

use platform::{
    contract,
    dispatcher::{AlarmsDispatcher, Id},
    message::Response as MessageResponse,
};
use sdk::{
    cosmwasm_std::{Addr, Env, QuerierWrapper, Storage, Timestamp, Uint64},
    cw_storage_plus::Deque,
};
use time_oracle::{Alarms, TimeSeconds};

use crate::{
    msg::{AlarmsCount, AlarmsStatusResponse, ExecuteAlarmMsg},
    result::ContractResult,
    ContractError,
};

pub(super) struct TimeAlarms {
    time_alarms: Alarms<'static>,
}

impl TimeAlarms {
    const ALARMS_NAMESPACE: &'static str = "alarms";
    const ALARMS_IDX_NAMESPACE: &'static str = "alarms_idx";
    const REPLY_ID: Id = 0;
    const EVENT_TYPE: &'static str = "timealarm";

    pub fn new() -> Self {
        Self {
            time_alarms: Alarms::new(Self::ALARMS_NAMESPACE, Self::ALARMS_IDX_NAMESPACE),
        }
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), ContractError> {
        self.time_alarms.remove(storage, addr)?;
        Ok(())
    }

    pub fn try_add_new(
        &self,
        querier: &QuerierWrapper<'_>,
        storage: &mut dyn Storage,
        env: &Env,
        address: Addr,
        time: Timestamp,
    ) -> ContractResult<MessageResponse> {
        if time < env.block.time {
            return Err(ContractError::InvalidAlarm(time));
        }

        contract::validate_addr(querier, &address)
            .map_err(ContractError::from)
            .and_then(|()| {
                self.time_alarms
                    .add(storage, address, time)
                    .map_err(Into::into)
            })
            .map(|()| Default::default())
    }

    pub fn try_notify(
        &self,
        storage: &mut dyn Storage,
        ctime: Timestamp,
        max_count: AlarmsCount,
    ) -> ContractResult<(AlarmsCount, MessageResponse)> {
        self.time_alarms
            .alarms_selection(storage, ctime)
            .take(max_count.try_into()?)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .try_fold(
                AlarmsDispatcher::new(ExecuteAlarmMsg::TimeAlarm {}, Self::EVENT_TYPE),
                |mut dispatcher, (addr, time)| -> ContractResult<_> {
                    dispatcher = dispatcher.send_to(&addr, Self::REPLY_ID)?;

                    self.time_alarms.remove(storage, addr.clone())?;

                    InDelivery::add(storage, addr, time)?;

                    Ok(dispatcher)
                },
            )
            .map(|dispatcher| (dispatcher.nb_sent(), dispatcher.into()))
    }

    pub fn try_any_alarm(
        &self,
        storage: &dyn Storage,
        ctime: Timestamp,
    ) -> Result<AlarmsStatusResponse, ContractError> {
        let remaining_alarms = self
            .time_alarms
            .alarms_selection(storage, ctime)
            .next()
            .transpose()?
            .is_some();
        Ok(AlarmsStatusResponse { remaining_alarms })
    }

    fn try_add_old(
        &self,
        storage: &mut dyn Storage,
        address: Addr,
        time: TimeSeconds,
    ) -> ContractResult<()> {
        self.time_alarms
            .add(storage, address, Timestamp::from_seconds(time))
            .map_err(Into::into)
    }
}

pub(super) struct InDelivery;

impl InDelivery {
    const ALARMS: Deque<'_, TimeAlarm> = Deque::new("alarms_in_delivery");

    fn add(storage: &mut dyn Storage, addr: Addr, time: TimeSeconds) -> ContractResult<()> {
        Self::ALARMS
            .push_back(
                storage,
                &TimeAlarm {
                    addr,
                    time: Uint64::new(time),
                },
            )
            .map_err(Into::into)
    }

    pub fn delivered(storage: &mut dyn Storage) -> ContractResult<()> {
        Self::ALARMS
            .pop_front(storage)
            .map(|maybe_alarm: Option<TimeAlarm>| debug_assert!(maybe_alarm.is_some()))
            .map_err(Into::into)
    }

    pub fn failed(storage: &mut dyn Storage) -> ContractResult<()> {
        Self::ALARMS
            .pop_front(storage)
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<TimeAlarm>| {
                maybe_alarm.ok_or(ContractError::ReplyOnEmptyAlarmQueue)
            })
            .and_then(|TimeAlarm { addr, time }| {
                TimeAlarms::new().try_add_old(storage, addr, time.u64())
            })
    }
}

#[derive(Serialize, Deserialize)]
struct TimeAlarm {
    addr: Addr,
    time: Uint64,
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
        let deps = deps.as_mut();
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let msg_sender = Addr::unchecked("some address");
        assert!(TimeAlarms::new()
            .try_add_new(
                &deps.querier,
                deps.storage,
                &env,
                msg_sender.clone(),
                Timestamp::from_nanos(8),
            )
            .is_err());

        let expected_error: ContractError = contract::validate_addr(&deps.querier, &msg_sender)
            .unwrap_err()
            .into();

        let result = TimeAlarms::new()
            .try_add_new(
                &deps.querier,
                deps.storage,
                &env,
                msg_sender,
                Timestamp::from_nanos(8),
            )
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
        assert!(TimeAlarms::new()
            .try_add_new(
                &deps.querier,
                deps.storage,
                &env,
                msg_sender,
                Timestamp::from_nanos(4),
            )
            .is_ok());
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
        TimeAlarms::new()
            .try_add_new(
                &deps.querier,
                deps.storage,
                &env,
                msg_sender,
                Timestamp::from_nanos(4),
            )
            .unwrap_err();
    }
}
