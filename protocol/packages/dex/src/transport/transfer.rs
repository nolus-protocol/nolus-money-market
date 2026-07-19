use currency::Group;
use finance::{coin::CoinDTO, instant::Instant};
use platform::batch::Batch;

use crate::SwapTask;

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
