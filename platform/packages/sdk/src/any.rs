use cosmos_sdk_proto::{
    Any as GoogleProtobufAny,
    prost::{DecodeError, Message},
};
use cosmwasm_std::Binary;
use serde::{Deserialize, Serialize};

/// A protobuf serialized message
///
/// A few reasons to define it:
/// - To provide serialization that is consistent with the deserialization at `wasmbinding/bindings@cosmos-sdk`,
///   particularly how `type_url` is represented on the wire. `GoogleProtobufAny` serializes it as `typeUrl`.
/// - To provide a better encapsulated type to carry responses
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProtobufAny {
    type_url: String,
    value: Binary,
}

impl ProtobufAny {
    /// Create a new ProtobufAny instance
    /// **type_url** is the Protobuf type URL of the serialized message
    /// **value** is the Protobuf serialized message
    pub fn new<Url, ProtoData>(type_url: Url, value: ProtoData) -> Self
    where
        Url: Into<String>,
        ProtoData: Into<Binary>,
    {
        ProtobufAny {
            type_url: type_url.into(),
            value: value.into(),
        }
    }

    pub fn of_type(&self, type_url: &str) -> bool {
        self.type_url == type_url
    }

    pub fn decode<M>(&self) -> Result<M, DecodeError>
    where
        M: Message + Default,
    {
        M::decode(self.value.as_slice())
    }
}

impl From<GoogleProtobufAny> for ProtobufAny {
    fn from(google_any: GoogleProtobufAny) -> Self {
        ProtobufAny::new(google_any.type_url, google_any.value)
    }
}

#[cfg(feature = "testing")]
impl From<ProtobufAny> for GoogleProtobufAny {
    fn from(any: ProtobufAny) -> Self {
        Self {
            type_url: any.type_url,
            value: any.value.into(),
        }
    }
}
