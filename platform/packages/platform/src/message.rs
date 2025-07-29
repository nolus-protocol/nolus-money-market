use crate::batch::{Batch, Emitter};

#[derive(Default)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct Response {
    pub(super) messages: Batch,
    pub(super) events: Vec<Emitter>,
}

impl Response {
    pub fn messages_only(messages: Batch) -> Self {
        Self {
            messages,
            events: vec![],
        }
    }

    pub fn messages_with_event(messages: Batch, event: Emitter) -> Self {
        Self {
            messages,
            events: vec![event],
        }
    }

    pub fn merge_with<R>(mut self, other: R) -> Self
    where
        R: Into<Self>,
    {
        let mut other = other.into();
        self.messages = self.messages.merge(other.messages);
        self.events.append(&mut other.events);
        self
    }
}

impl From<Batch> for Response {
    fn from(messages: Batch) -> Self {
        Self::messages_only(messages)
    }
}

impl From<Emitter> for Response {
    fn from(event: Emitter) -> Self {
        Self::messages_with_event(Default::default(), event)
    }
}
