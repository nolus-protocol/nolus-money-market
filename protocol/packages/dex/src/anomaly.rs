use std::fmt::{Display, Formatter, Result as FmtResult};

use platform::remote::ErrorDetails as RemoteErrorDetails;

use crate::SwapTask;

/// Why a swap was refused, as coarsely as the workflow needs to know
///
/// Deliberately coarser than any counterparty's own error taxonomy: this crate
/// has exactly two treatments to choose between, so a finer split would name no
/// distinct behaviour. Owning the type here rather than re-exporting the
/// transport's keeps `dex` independent of a particular counterparty's error
/// vocabulary.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "testing", derive(Debug, PartialEq, Eq))]
pub enum Cause {
    /// The swap executed but could not reach the requested minimum output.
    MinOutputNotFulfilled,
    /// Any other refusal the counterparty reported.
    Other,
}

/// A refused swap: why it was refused, and the counterparty's own account of it
///
/// The workflow branches on [`Self::cause`]; the details are carried for
/// operator-facing troubleshooting and reach the reader through [`Display`],
/// which renders exactly what the bare counterparty response used to.
///
/// Intentionally not serialisable. The error path runs synchronously inside a
/// single message execution and never hops through `ResponseDelivery`, so this
/// value crosses no message boundary and adds nothing to any persisted layout.
pub struct ErrorAck {
    cause: Cause,
    details: RemoteErrorDetails,
}

impl ErrorAck {
    pub const fn new(cause: Cause, details: RemoteErrorDetails) -> Self {
        Self { cause, details }
    }

    pub const fn cause(&self) -> Cause {
        self.cause
    }
}

impl Display for ErrorAck {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.details.fmt(f)
    }
}

/// The options how a detected anomaly should be treated
///
/// They include a retry of the failed swap, or moving out to another state.
pub enum Treatment<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    Retry(SwapTaskT),
    Exit(SwapTaskT::Result),
}

/// Decide how a detected anomaly should be treated
///
/// Usually the swap specification plays that role.
pub trait Handler<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    /// `cause` is advisory: a leg is free to ignore it and treat every anomaly
    /// alike, which is what a leg without a meaningful output floor does.
    fn on_anomaly(self, cause: Cause) -> Treatment<SwapTaskT>;
}

#[cfg(test)]
mod test {
    use platform::remote::ErrorDetails as RemoteErrorDetails;

    use super::{Cause, ErrorAck};

    #[test]
    fn cause_survives_the_wrapper() {
        assert!(matches!(
            ErrorAck::new(Cause::MinOutputNotFulfilled, details()).cause(),
            Cause::MinOutputNotFulfilled
        ));
        assert!(matches!(
            ErrorAck::new(Cause::Other, details()).cause(),
            Cause::Other
        ));
    }

    // Threading `ErrorAck` through `on_error` left every existing error message
    // untouched only because the rendering stays the bare response. Pin it, so a
    // later urge to prepend the cause has to break this test first.
    #[test]
    fn display_renders_the_response_alone() {
        assert_eq!(
            details().to_string(),
            ErrorAck::new(Cause::MinOutputNotFulfilled, details()).to_string(),
        );
    }

    fn details() -> RemoteErrorDetails {
        RemoteErrorDetails::from("dex pool drained".to_owned())
    }
}
