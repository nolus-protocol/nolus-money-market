use prost::Message;

use sdk::{
    cosmos_sdk_proto::cosmos::base::abci::v1beta1::{MsgData, TxMsgData},
    neutron_sdk::bindings::types::ProtobufAny,
};

use crate::error::{Error, Result};

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

pub fn decode_msg_responses(data: &[u8]) -> Result<impl Iterator<Item = MsgData>> {
    Ok(TxMsgData::decode(data)?.data.into_iter())
}

#[cfg(feature = "testing")]
pub fn encode_msg_responses<I>(msgs: I) -> Vec<u8>
where
    I: Iterator<Item = MsgData>,
{
    let tx = TxMsgData {
        data: msgs.collect(),
    };
    tx.encode_to_vec()
}

pub fn decode_msg_response<T, M>(resp: MsgData, msg_type: T) -> Result<M>
where
    T: Into<String>,
    M: Message + Default,
{
    let msg_type = msg_type.into();

    if resp.msg_type != msg_type {
        return Err(Error::ProtobufInvalidType(msg_type, resp.msg_type));
    }
    M::decode(resp.data.as_slice()).map_err(Into::into)
}

pub fn encode_msg_response<T, M>(resp: M, msg_type: T) -> MsgData
where
    T: Into<String>,
    M: Message + Default,
{
    MsgData {
        msg_type: msg_type.into(),
        data: resp.encode_to_vec(),
    }
}
