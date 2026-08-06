//! The paired ICS-20 transfer channel, and the **handshake** version that
//! proposes it.
//!
//! The handshake version carries one datum the packet layer never sees: the
//! ICS-20 transfer channel the counterparty must pair with this lease channel.
//! It is therefore `<protocol version>+transfer=channel-<n>`, while every
//! packet still carries the bare [`crate::VERSION`] through
//! [`crate::version::ProtocolVersion`]. Mixing the two is a protocol error in
//! both directions: a suffixed version on a packet fails the envelope
//! deserialiser, and a bare version in the handshake fails
//! [`Ics20ChannelId::from_channel_version`].
//!
//! Both ends consume this module, which is what keeps the two repositories'
//! renderings byte-identical.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{VERSION, error::Error};

/// Maximum byte length of an ICS-20 channel identifier this grammar accepts.
///
/// The ordinal is bounded by `u16` — the counterparty derives a program address
/// from it at that width — so five digits is the widest canonical rendering.
pub const ICS20_CHANNEL_ID_MAX_BYTES: usize = ICS20_CHANNEL_PREFIX.len() + U16_MAX_DIGITS;

/// Maximum byte length of a whole handshake version string.
///
/// Everything this grammar can render fits within it, so a longer input is
/// rejected before any of it is retained in an error or an event.
pub const CHANNEL_VERSION_MAX_BYTES: usize =
    VERSION.len() + CHANNEL_VERSION_TRANSFER_TAG.len() + ICS20_CHANNEL_ID_MAX_BYTES;

const CHANNEL_VERSION_TRANSFER_TAG: &str = "+transfer=";

const ICS20_CHANNEL_PREFIX: &str = "channel-";

const U16_MAX_DIGITS: usize = u16_max_digits();

// `usize::try_from` is not const-stable; the widening cast is isolated here.
const fn u16_max_digits() -> usize {
    (u16::MAX.ilog10() + 1) as usize
}

/// A canonical ICS-20 channel identifier.
///
/// The ordinal is the only state, so a non-canonical value is unrepresentable
/// and a parse-then-render round-trip is the identity. Parsing is the sole way
/// in from a string and doubles as the deserialiser, so a message carrying
/// `channel-01`, `channel-65536`, or `Channel-1` is refused at decode rather
/// than by business code downstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ics20ChannelId(u16);

impl Ics20ChannelId {
    /// Compose the handshake version proposing this channel as the paired
    /// transfer channel.
    ///
    /// Infallible: a value of this type is canonical by construction, so there
    /// is no way to compose a version string the counterparty would refuse.
    pub fn channel_version(&self) -> String {
        format!("{VERSION}{CHANNEL_VERSION_TRANSFER_TAG}{self}")
    }

    /// Recover the proposed channel from a handshake version.
    ///
    /// The inverse of [`Self::channel_version`]: it accepts exactly what that
    /// method renders and nothing else, so a round-trip over the accepted set
    /// pins both halves of the grammar at once.
    pub fn from_channel_version(version: &str) -> Result<Self, Error> {
        require_within_cap(version)
            .and_then(|()| {
                version
                    .strip_prefix(VERSION)
                    .and_then(|rest| rest.strip_prefix(CHANNEL_VERSION_TRANSFER_TAG))
                    .ok_or(Error::ChannelVersionMalformed)
            })
            .and_then(|ics20_channel| {
                ics20_channel
                    .parse()
                    .map_err(|_| Error::ChannelVersionMalformed)
            })
    }
}

impl FromStr for Ics20ChannelId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let len = value.len();
        if ICS20_CHANNEL_ID_MAX_BYTES < len {
            return Err(Error::Ics20ChannelIdTooLong {
                actual: len,
                max: ICS20_CHANNEL_ID_MAX_BYTES,
            });
        }
        value
            .strip_prefix(ICS20_CHANNEL_PREFIX)
            .filter(|ordinal| is_canonical(ordinal))
            .and_then(|ordinal| ordinal.parse().ok())
            .map(Self)
            .ok_or(Error::Ics20ChannelIdNonCanonical)
    }
}

impl fmt::Display for Ics20ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ICS20_CHANNEL_PREFIX)
            .and_then(|()| fmt::Display::fmt(&self.0, f))
    }
}

impl Serialize for Ics20ChannelId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Ics20ChannelId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Ics20ChannelIdVisitor)
    }
}

struct Ics20ChannelIdVisitor;

impl de::Visitor<'_> for Ics20ChannelIdVisitor {
    type Value = Ics20ChannelId;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a canonical ICS-20 channel id '{ICS20_CHANNEL_PREFIX}<n>' with <n> within u16"
        )
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value
            .parse()
            .map_err(|err: Error| E::custom(err.to_string()))
    }
}

/// Truncate a counterparty-supplied version to a length safe to retain in an
/// error or an event, on a UTF-8 character boundary.
///
/// The handshake reports a rejected version string for diagnostics; this is
/// what keeps that echo bounded by our own grammar rather than by the
/// counterparty's input. It is deliberately not a parser — a rejected version
/// has no structure to rely on.
pub fn bounded_channel_version(version: &str) -> &str {
    if version.len() <= CHANNEL_VERSION_MAX_BYTES {
        version
    } else {
        &version[..floor_char_boundary(version, CHANNEL_VERSION_MAX_BYTES)]
    }
}

fn require_within_cap(version: &str) -> Result<(), Error> {
    let len = version.len();
    if CHANNEL_VERSION_MAX_BYTES < len {
        Err(Error::ChannelVersionTooLong {
            actual: len,
            max: CHANNEL_VERSION_MAX_BYTES,
        })
    } else {
        Ok(())
    }
}

fn is_canonical(ordinal: &str) -> bool {
    !ordinal.is_empty()
        && ordinal.bytes().all(|byte| byte.is_ascii_digit())
        && (ordinal == "0" || !ordinal.starts_with('0'))
}

fn floor_char_boundary(value: &str, max: usize) -> usize {
    (0..=max)
        .rev()
        .find(|&index| value.is_char_boundary(index))
        .unwrap_or_default()
}
