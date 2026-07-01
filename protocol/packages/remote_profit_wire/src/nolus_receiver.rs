use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::error::Error;

/// Maximum byte length of a [`NolusReceiver`] payload.
///
/// BIP-173 caps a bech32 string at 90 characters; a Nolus account address
/// (`nolus1…`, 32-byte witness) is 63 characters. The 90-byte cap keeps the
/// upper BIP-173 bound while still bounding event and storage size from a
/// possibly-misbehaving counterparty.
pub const NOLUS_RECEIVER_MAX_BYTES: usize = 90;

/// Human-readable part every Nolus address carries before the bech32 separator.
const NOLUS_HRP: &str = "nolus";

/// bech32 data-part alphabet (BIP-173). Excludes `1`, `b`, `i`, `o`.
const BECH32_CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Number of trailing bech32 symbols reserved for the BIP-173 checksum. The
/// data part must carry at least this many symbols, or there is no address
/// payload left once the checksum is accounted for.
const BECH32_CHECKSUM_LEN: usize = 6;

/// Typed wrapper around the Nolus address the funded profit drains into,
/// travelling on the wire at `open_profit` so the Solana side learns the
/// store-once receiver up front (subsequent `TransferOut` packets stay
/// amount-only, recipient derived from the stored value).
///
/// Serialises as a bare JSON string so an off-chain consumer reads the field
/// as a plain bech32 address. Validation lives inside the constructor: the
/// `nolus` human-readable part, the bech32 character set, and the BIP-173
/// checksum. Deserialisation enforces the same invariants — a packet with an
/// invalid receiver is rejected at parse time, never observed by business
/// code. Checksum validity is a wire-shape gate, not authorisation; a Nolus
/// consumer still resolves the string through its own `Addr` constructor
/// before dispatching against it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NolusReceiver(String);

impl NolusReceiver {
    pub fn new<S>(value: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let value: String = value.into();
        validate(&value)
            .map(|()| Self(value))
            .inspect(|receiver| debug_assert!(receiver.invariant_held()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn invariant_held(&self) -> bool {
        validate(&self.0).is_ok()
    }
}

impl fmt::Display for NolusReceiver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for NolusReceiver {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

fn validate(value: &str) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::NolusReceiverEmpty);
    }
    let len = value.len();
    if NOLUS_RECEIVER_MAX_BYTES < len {
        return Err(Error::NolusReceiverTooLong {
            actual: len,
            max: NOLUS_RECEIVER_MAX_BYTES,
        });
    }

    // bech32 forbids mixing upper- and lower-case (BIP-173). Nolus addresses
    // are lower-case; reject anything that is not uniformly lower-case so an
    // upper- or mixed-case string never slips past the checksum.
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(Error::NolusReceiverInvalidBech32);
    }

    let separator = value.rfind('1').ok_or(Error::NolusReceiverInvalidBech32)?;

    let hrp = &value[..separator];
    if hrp != NOLUS_HRP {
        return Err(Error::NolusReceiverWrongHrp);
    }

    let data = &value[separator + 1..];
    // The data part holds at least the checksum symbols.
    if data.len() < BECH32_CHECKSUM_LEN {
        return Err(Error::NolusReceiverInvalidBech32);
    }

    let symbols = data
        .bytes()
        .map(|byte| {
            BECH32_CHARSET
                .iter()
                .position(|candidate| *candidate == byte)
                .ok_or(Error::NolusReceiverInvalidBech32)
                .and_then(|symbol| {
                    u8::try_from(symbol).map_err(|_| Error::NolusReceiverInvalidBech32)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    checksum_holds(hrp, &symbols)
        .then_some(())
        .ok_or(Error::NolusReceiverInvalidBech32)
}

/// BIP-173 checksum: the polymod over the expanded HRP plus the data symbols
/// (which already include the trailing 6 checksum symbols) must equal 1.
fn checksum_holds(hrp: &str, data_symbols: &[u8]) -> bool {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data_symbols);
    polymod(&values) == 1
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut values: Vec<u8> = hrp.bytes().map(|byte| byte >> 5).collect();
    values.push(0);
    values.extend(hrp.bytes().map(|byte| byte & 0x1f));
    values
}

fn polymod(values: &[u8]) -> u32 {
    const GENERATOR: [u32; 5] = [
        0x3b6a_57b2,
        0x2650_8e6d,
        0x1ea1_19fa,
        0x3d42_33dd,
        0x2a14_62b3,
    ];

    values.iter().fold(1u32, |checksum, &value| {
        let top = checksum >> 25;
        let mut next = ((checksum & 0x01ff_ffff) << 5) ^ u32::from(value);
        for (bit, generator) in GENERATOR.iter().enumerate() {
            if (top >> bit) & 1 == 1 {
                next ^= generator;
            }
        }
        next
    })
}

impl Serialize for NolusReceiver {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for NolusReceiver {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(NolusReceiverVisitor)
    }
}

struct NolusReceiverVisitor;

impl de::Visitor<'_> for NolusReceiverVisitor {
    type Value = NolusReceiver;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a bech32 Nolus address (`{NOLUS_HRP}1…`) of at most {NOLUS_RECEIVER_MAX_BYTES} bytes"
        )
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        NolusReceiver::new(value).map_err(|err| E::custom(err.to_string()))
    }
}
