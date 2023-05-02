use currency::native::Nls;
use platform::batch::{Batch, Emit, Emitter};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::Addr;

use crate::{msg::ExecuteAlarmMsg, ContractError};

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

    pub fn nb_sent(&self) -> u32 {
        self.batch
            .len()
            .try_into()
            .expect("used with alarms less than or equal to AlarmsCount::MAX")
    }
}

impl From<OracleAlarmDispatcher> for MessageResponse {
    fn from(value: OracleAlarmDispatcher) -> Self {
        MessageResponse::messages_with_events(value.batch, value.emitter)
    }
}

#[cfg(test)]
mod test {
    use platform::response;
    use sdk::{
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{Addr, Event, ReplyOn},
    };

    use crate::dispatcher::{EVENT_KEY, EVENT_TYPE};

    use super::*;

    #[test]
    fn empty() {
        let d: CwResponse = response::response_only_messages(OracleAlarmDispatcher::new());
        assert!(!d.events.is_empty());
        assert_eq!(Event::new(EVENT_TYPE), d.events[0]);
        assert!(d.messages.is_empty());
    }

    #[test]
    fn one_alarm() {
        let d = OracleAlarmDispatcher::new();
        let receiver = Addr::unchecked("time_alarm receiver");

        let d = d.send_to(Id::MAX, receiver.clone()).unwrap();
        let r: CwResponse = response::response_only_messages(d);
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
