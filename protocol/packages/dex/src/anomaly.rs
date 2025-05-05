use crate::SwapTask;

pub enum Treatment<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    Retry(SwapTaskT),
    Exit(SwapTaskT::Result),
}
