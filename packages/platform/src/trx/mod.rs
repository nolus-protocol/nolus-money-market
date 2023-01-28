use prost::Message;
use sdk::neutron_sdk::bindings::types::ProtobufAny;

#[derive(Default)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct Transaction {
    msgs: Vec<ProtobufAny>,
}

impl Transaction {
    pub fn add_message<T, M>(&mut self, msg_type: T, msg: M)
    where
        T: Into<String>,
        M: Message,
    {
        let mut buf = Vec::with_capacity(msg.encoded_len());
        msg.encode_raw(&mut buf);

        self.msgs
            .push(ProtobufAny::new(msg_type.into(), buf.into()));
    }

    pub(super) fn into_msgs(self) -> Vec<ProtobufAny> {
        self.msgs
    }
}
