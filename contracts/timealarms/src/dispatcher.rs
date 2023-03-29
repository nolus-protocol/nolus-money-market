use crate::{
    msg::{AlarmsCount, DispatchAlarmsResponse, ExecuteAlarmMsg},
    ContractError,
};
use currency::native::Nls;
use platform::batch::{Batch, Emit, Emitter};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{self, Addr},
};

pub type Id = u64;

pub(super) struct OracleAlarmDispatcher {
    batch: Batch,
    emitter: Emitter,
}

const EVENT_TYPE: &str = "timealarm";
const EVENT_KEY: &str = "receiver";

impl OracleAlarmDispatcher {
    pub fn new() -> Self {
        Self {
            batch: Default::default(),
            emitter: Emitter::of_type(EVENT_TYPE),
        }
    }

    pub fn send_to(mut self, id: Id, addr: Addr) -> Result<Self, ContractError> {
        self.emitter = self.emitter.emit(EVENT_KEY, &addr);

        self.batch.schedule_execute_wasm_reply_always::<_, Nls>(
            &addr,
            ExecuteAlarmMsg::TimeAlarm {},
            None,
            id,
        )?;

        Ok(self)
    }
}

impl TryFrom<OracleAlarmDispatcher> for Response {
    type Error = ContractError;
    fn try_from(value: OracleAlarmDispatcher) -> Result<Self, Self::Error> {
        let msgs: AlarmsCount = value
            .batch
            .len()
            .try_into()
            .expect("used with alarms less than or equal to AlarmsCount::MAX");

        Ok(value
            .batch
            .into_response(value.emitter)
            .set_data(cosmwasm_std::to_binary(&DispatchAlarmsResponse(msgs))?))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::dispatcher::{EVENT_KEY, EVENT_TYPE};

    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{Addr, Event, ReplyOn},
    };

    #[test]
    fn empty() {
        let d = OracleAlarmDispatcher::new();
        let r: Response = d.try_into().unwrap();
        assert!(!r.events.is_empty());
        assert_eq!(Event::new(EVENT_TYPE), r.events[0]);
        assert!(r.messages.is_empty());
    }

    #[test]
    fn one_alarm() {
        let d = OracleAlarmDispatcher::new();
        let receiver = Addr::unchecked("time_alarm receiver");

        let d = d.send_to(Id::MAX, receiver.clone()).unwrap();
        let r: Response = d.try_into().unwrap();
        assert!(!r.events.is_empty());
        assert_eq!(
            Event::new(EVENT_TYPE).add_attribute(EVENT_KEY, receiver),
            r.events[0]
        );

        assert!(!r.messages.is_empty());
        let msg = &r.messages[0];
        assert_eq!(ReplyOn::Always, msg.reply_on);
    }
}
