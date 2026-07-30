use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{error::Error, response::OperationResponse};

/// Maximum byte length of a [`RemoteError::message`] payload.
///
/// Why a cap: the string is authored by the Solana counterparty and consumed
/// by Nolus storage / event emission. Without a bound, a hostile or
/// misbehaving counterparty can inflate event sizes and storage rows
/// arbitrarily. 200 bytes leaves ample room for the counterparty's prose once
/// the [code frame](RemoteError::parse_ack) is stripped, while forbidding
/// abuse.
pub const OPERATION_ERR_MAX_BYTES: usize = 200;

/// Maximum byte length of the code token itself, excluding its delimiters.
///
/// Bounds the scan in [`RemoteError::parse_ack`], so a hostile 200-byte
/// acknowledgement cannot make the parser walk its whole length, and bounds
/// the token retained in [`Error::CallbackErrorCodeUnknown`].
pub const REMOTE_ERROR_CODE_MAX_BYTES: usize = 16;

/// Maximum byte length of the whole code frame: `[`, the token, `]`, one space.
pub const REMOTE_ERROR_CODE_FRAME_MAX_BYTES: usize =
    REMOTE_ERROR_CODE_MAX_BYTES + CODE_FRAME_DELIMITER_BYTES;

const CODE_OPEN: u8 = b'[';
const CODE_CLOSE: u8 = b']';
const CODE_SEPARATOR: u8 = b' ';

// The two brackets around the token, without the space that follows them.
const CODE_BRACKET_BYTES: usize = 2;

// Everything in the frame that is not the token: both brackets and the
// separating space.
const CODE_FRAME_DELIMITER_BYTES: usize = CODE_BRACKET_BYTES + 1;

// What the counterparty appends in place of the prose it drops when truncating.
const COUNTERPARTY_TRUNCATION_MARKER_BYTES: usize = 3;

// The counterparty renders the frame ahead of its prose and truncates the tail
// to `OPERATION_ERR_MAX_BYTES` with a marker. Unless the cap exceeds a full
// frame plus that marker, truncation could eat the code itself.
const _: () = assert!(
    REMOTE_ERROR_CODE_FRAME_MAX_BYTES + COUNTERPARTY_TRUNCATION_MARKER_BYTES
        < OPERATION_ERR_MAX_BYTES
);

/// Outcome of a remote operation as reported back to the Nolus controller.
///
/// `OperationOk` carries the typed response when Solana confirmed the
/// requested action. `OperationErr` carries the counterparty's classified
/// failure. `OperationTimeout` is emitted by the IBC layer when the packet was
/// never acknowledged — it is structurally distinct from `OperationErr`
/// because the recovery path differs (funds may still be in flight on the
/// Solana side until the channel times out).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteLeaseCallback {
    OperationOk(OperationResponse),
    OperationErr(RemoteError),
    OperationTimeout,
}

/// Why the counterparty rejected a remote operation.
///
/// The machine-readable half of [`RemoteError`]. Consumers branch on this and
/// never on [`RemoteError::message`]: the counterparty's rendered prose is not
/// a stable key — it tracks an upstream DEX API whose wording changes across
/// minor releases — whereas these tokens are an append-only contract.
///
/// Serialises to the same tokens [`Self::as_wire`] returns, which is what lets
/// the counterparty frame an acknowledgement and this crate parse it with one
/// table. An unrecognised token is **rejected**, never mapped to a default —
/// see [`RemoteError::parse_ack`].
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteErrorKind {
    /// The swap could not fulfil the requested minimum output.
    MinOutUnmet,
    /// A deterministic refusal of the request itself: identical bytes fail again.
    Permanent,
    /// A stale counterparty view; a fresh emission may pass.
    Transient,
}

impl RemoteErrorKind {
    const ALL: [Self; 3] = [Self::MinOutUnmet, Self::Permanent, Self::Transient];

    /// The token this kind occupies on the wire.
    ///
    /// The single source of truth for the vocabulary: the serde representation
    /// is pinned equal to it, and [`Self::try_from_wire`] is derived from it.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::MinOutUnmet => "min_out_unmet",
            Self::Permanent => "permanent",
            Self::Transient => "transient",
        }
    }

    fn try_from_wire(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_wire() == token)
    }

    const fn is_token_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
    }
}

/// A counterparty-reported failure: a classified [`RemoteErrorKind`] plus the
/// counterparty's length-capped human-readable message.
///
/// On the acknowledgement wire the two travel as one string, the kind framed
/// ahead of the prose — see [`Self::format_ack`] and [`Self::parse_ack`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RemoteError {
    kind: RemoteErrorKind,
    message: RemoteErrorMessage,
}

impl RemoteError {
    pub const fn new(kind: RemoteErrorKind, message: RemoteErrorMessage) -> Self {
        Self { kind, message }
    }

    pub const fn kind(&self) -> RemoteErrorKind {
        self.kind
    }

    pub const fn message(&self) -> &RemoteErrorMessage {
        &self.message
    }

    /// For consumers that keep only the prose, e.g. a terminal state that
    /// stores the reason it was entered with.
    pub fn into_message(self) -> RemoteErrorMessage {
        self.message
    }

    /// Render `kind` and `message` into the single string an IBC error
    /// acknowledgement carries.
    ///
    /// The frame precedes the message because the counterparty truncates the
    /// **tail** to fit [`OPERATION_ERR_MAX_BYTES`]: at offset zero the code
    /// survives truncation structurally rather than by luck. It also keeps this
    /// crate ignorant of whatever provenance prefix the counterparty puts on
    /// its own prose.
    pub fn format_ack(kind: RemoteErrorKind, message: &str) -> String {
        let token = kind.as_wire();
        let open = char::from(CODE_OPEN);
        let close = char::from(CODE_CLOSE);
        format!("{open}{token}{close} {message}")
    }

    /// Parse the string carried by an IBC error acknowledgement.
    ///
    /// Both failure modes are counterparty non-conformance, and both are
    /// rejected rather than guessed: an absent or malformed frame, or a token
    /// this build does not know. Mapping an unrecognised code onto some default
    /// would invent a meaning the counterparty never sent and then route funds
    /// with it; the two ends of this protocol move in lockstep, so a code that
    /// does not parse is a deployment fault to fix, not a case to absorb.
    ///
    /// The kind is stripped from the retained message: it is typed in
    /// [`Self::kind`], and leaving it in the prose invites a downstream
    /// consumer to parse text.
    pub fn parse_ack<S>(ack: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let ack: String = ack.into();

        Self::split_code(&ack).and_then(|ParsedCode { kind, prose_at }| {
            RemoteErrorMessage::new(&ack[prose_at..]).map(|message| Self::new(kind, message))
        })
    }

    fn split_code(ack: &str) -> Result<ParsedCode, Error> {
        Self::scan_token(ack)
            .ok_or(Error::CallbackErrorCodeMissing {
                max: REMOTE_ERROR_CODE_MAX_BYTES,
            })
            .and_then(|token| {
                RemoteErrorKind::try_from_wire(token)
                    .ok_or_else(|| Error::CallbackErrorCodeUnknown {
                        code: token.to_owned(),
                    })
                    .map(|kind| ParsedCode {
                        kind,
                        prose_at: Self::prose_offset(ack, token.len()),
                    })
            })
    }

    // A single `None` covers every malformed frame alike — no opening bracket,
    // no closing one within the token cap, a slice landing off a char boundary,
    // an empty token, a byte outside the token alphabet — because the caller
    // reports them all as one non-conformance.
    fn scan_token(ack: &str) -> Option<&str> {
        ack.strip_prefix(char::from(CODE_OPEN))
            .and_then(|framed| {
                // One byte past the cap: enough to tell "over-long token" from
                // "no closing delimiter at all", without scanning the rest of
                // the string.
                let window = framed
                    .len()
                    .min(REMOTE_ERROR_CODE_MAX_BYTES.saturating_add(1));

                framed.as_bytes()[..window]
                    .iter()
                    .position(|&byte| byte == CODE_CLOSE)
                    .and_then(|close_at| framed.get(..close_at))
            })
            .filter(|token| !token.is_empty() && token.bytes().all(RemoteErrorKind::is_token_byte))
    }

    // The prose starts past `[` + token + `]`, plus the single separating space
    // when the counterparty sent one. Every comparison is ASCII, so the result
    // is a char boundary even when the prose is not.
    fn prose_offset(ack: &str, token_len: usize) -> usize {
        let after_frame = token_len + CODE_BRACKET_BYTES;

        after_frame + usize::from(ack.as_bytes().get(after_frame) == Some(&CODE_SEPARATOR))
    }
}

struct ParsedCode {
    kind: RemoteErrorKind,
    prose_at: usize,
}

/// Length-capped error string returned by the Solana counterparty.
///
/// Serialises as a bare JSON string. The counterparty-facing paths —
/// deserialisation and the fallible [`new`](Self::new) — reject payloads
/// above [`OPERATION_ERR_MAX_BYTES`], so any string sourced from over the
/// wire is bounded before it reaches downstream storage. The
/// [`from_static`](Self::from_static) constructor is the one exception: it
/// trusts the (compile-time-known) caller and only `debug_assert!`s the
/// bound, so it must be fed provably-in-range literals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteErrorMessage(String);

impl RemoteErrorMessage {
    pub fn new<S>(message: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let message: String = message.into();
        let actual = message.len();
        if actual <= OPERATION_ERR_MAX_BYTES {
            Ok(Self(message))
        } else {
            Err(Error::CallbackErrorTooLong {
                actual,
                max: OPERATION_ERR_MAX_BYTES,
            })
        }
    }

    /// Construct from a compile-time-known string literal that is statically
    /// known to be within [`OPERATION_ERR_MAX_BYTES`].
    ///
    /// For fixed internal reasons (e.g. `"timeout"`) where threading a
    /// fallible [`new`](Self::new) through the call site would add error
    /// plumbing for a value that is provably in range.
    ///
    /// Precondition (caller's responsibility): `message` must be a genuine
    /// literal whose length is verifiable by inspection, never a
    /// runtime-produced `&'static str` (e.g. a `Box::leak`ed value). The
    /// length is only `debug_assert!`ed, so an over-cap input that slips past
    /// review would bypass the bound in release builds — unlike [`new`] and
    /// deserialisation, which reject it. When the length is not statically
    /// obvious, use [`new`] instead.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `message` exceeds [`OPERATION_ERR_MAX_BYTES`].
    pub fn from_static(message: &'static str) -> Self {
        debug_assert!(
            message.len() <= OPERATION_ERR_MAX_BYTES,
            "RemoteErrorMessage::from_static exceeds OPERATION_ERR_MAX_BYTES"
        );
        Self(message.to_owned())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Serialize for RemoteErrorMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RemoteErrorMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Rejects payloads above the cap inside the visitor, before any
        // owned `String` is materialised on our side. With `serde_json`'s
        // `deserialize_str` the visitor receives a borrowed slice into the
        // input buffer when no JSON escapes are present, so the over-cap
        // case allocates nothing beyond the already-bounded IBC packet.
        deserializer.deserialize_str(RemoteErrorMessageVisitor)
    }
}

struct RemoteErrorMessageVisitor;

impl RemoteErrorMessageVisitor {
    fn take_within_cap<E, F>(&self, len: usize, take: F) -> Result<RemoteErrorMessage, E>
    where
        E: de::Error,
        F: FnOnce() -> String,
    {
        (len <= OPERATION_ERR_MAX_BYTES)
            .then(take)
            .map(RemoteErrorMessage)
            .ok_or_else(|| E::invalid_length(len, self))
    }
}

impl<'de> de::Visitor<'de> for RemoteErrorMessageVisitor {
    type Value = RemoteErrorMessage;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a string of at most {OPERATION_ERR_MAX_BYTES} bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.take_within_cap(value.len(), || value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let len = value.len();
        self.take_within_cap(len, || value)
    }
}
