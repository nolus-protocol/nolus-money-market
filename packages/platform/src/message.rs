use crate::batch::{Batch, Emitter};

#[derive(Default)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct Response {
    pub(super) messages: Batch,
    pub(super) events: Option<Emitter>,
}

impl Response {
    pub fn messages_only(messages: Batch) -> Response {
        Self {
            messages,
            events: None,
        }
    }

    pub fn messages_with_events(messages: Batch, events: Emitter) -> Response {
        Self {
            messages,
            events: Some(events),
        }
    }
}

impl From<Batch> for Response {
    fn from(messages: Batch) -> Self {
        Self::messages_only(messages)
    }
}

impl From<Emitter> for Response {
    fn from(events: Emitter) -> Self {
        Self::messages_with_events(Default::default(), events)
    }
}
