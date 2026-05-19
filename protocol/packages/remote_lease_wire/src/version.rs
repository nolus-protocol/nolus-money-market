use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::VERSION;

/// Zero-sized wire-format marker that always serialises as [`crate::VERSION`]
/// and rejects any other value at deserialisation time.
///
/// Why a ZST: every packet on the wire carries the protocol version explicitly,
/// but the value is fixed at compile time. A mismatched-version packet from
/// a future counterparty MUST be rejected at the deserialiser, not in the
/// consumer's business code, so that no callback dispatch can ever observe a
/// `PacketEnvelope` whose `version` field does not equal the current
/// `VERSION` constant. Bumping the protocol therefore forces a code change
/// here — exactly the desired invariant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProtocolVersion;

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(VERSION)
    }
}

impl Serialize for ProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(VERSION)
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <&str>::deserialize(deserializer).and_then(|actual| {
            if actual == VERSION {
                Ok(Self)
            } else {
                Err(de::Error::custom(format!(
                    "protocol version mismatch: expected {VERSION}, got {actual}",
                )))
            }
        })
    }
}
