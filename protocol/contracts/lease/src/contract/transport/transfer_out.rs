use serde::{Deserialize, Serialize};

use currency::Group;
use cw_time::IntoTimestamp;
use dex::{Connectable, IBC_TIMEOUT, SwapTask, TransportOut, TransportOutFactory};
use finance::{coin::CoinDTO, instant::Instant};
use platform::{bank_ibc::local::Sender as LocalSender, batch::Batch};

#[derive(Default, Serialize, Deserialize)]
pub struct TransferOutFactory {}

impl TransportOutFactory for TransferOutFactory {
    type Transport<'this> = TransferOutTrx<'this>;

    fn transport<'task, Task>(&self, task: &'task Task, now: Instant) -> Self::Transport<'task>
    where
        Task: SwapTask,
    {
        Self::Transport::new(LocalSender::new(
            &task.dex_account().dex().transfer_channel.local_endpoint,
            task.dex_account().owner(),
            task.dex_account().remote(),
            (now + IBC_TIMEOUT).into_timestamp(),
            format!(
                "Transfer out: {sender} -> {receiver}",
                sender = task.dex_account().owner(),
                receiver = task.dex_account().remote()
            ),
        ))
    }
}

pub struct TransferOutTrx<'account> {
    sender: LocalSender<'account>,
}

impl<'account> TransferOutTrx<'account> {
    fn new(sender: LocalSender<'account>) -> Self {
        Self { sender }
    }
}

impl<'account> TransportOut for TransferOutTrx<'account> {
    fn send<G>(&mut self, amount: &CoinDTO<G>)
    where
        G: Group,
    {
        self.sender.send(amount)
    }
}

impl<'account> From<TransferOutTrx<'account>> for Batch {
    fn from(value: TransferOutTrx<'account>) -> Self {
        value.sender.into()
    }
}
