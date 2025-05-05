use crate::SwapTask;

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
    fn on_anomaly(self) -> Treatment<SwapTaskT>;
}
