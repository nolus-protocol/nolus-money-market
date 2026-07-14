use currency::Group;
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::batch::Batch;

use crate::SwapTask;

/// IBC transfer timeout — long enough for relayers to process.
pub const IBC_TIMEOUT: Duration = Duration::from_days(1);

pub trait TransferOut
where
    Self: Into<Batch>,
{
    fn send<G>(&mut self, amount: &CoinDTO<G>)
    where
        G: Group;
}

pub trait TransferOutFactory {
    type Transport<'this>: TransferOut
    where
        Self: 'this;

    fn transport<'task, Task>(&self, task: &'task Task, now: Instant) -> Self::Transport<'task>
    where
        Task: SwapTask;
}
