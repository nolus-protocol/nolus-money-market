use finance::instant::Instant;

use crate::{SwapTask, swap::ExactAmountIn};

pub trait Factory {
    type Transport<'this>: ExactAmountIn
    where
        Self: 'this;

    fn transport<'task, Task>(&self, task: &'task Task, now: Instant) -> Self::Transport<'task>
    where
        Task: SwapTask;
}
