use serde::{Deserialize, Serialize};

use dex::{RemoteLeaseTransportFactory, SwapTask};
use finance::instant::Instant;
use swap::Impl as SwapClient;

#[derive(Default, Serialize, Deserialize)]
pub struct SwapClientFactory {}

impl RemoteLeaseTransportFactory for SwapClientFactory {
    type TransportImpl<'this> = SwapClient;

    fn transport<'task, Task>(
        &self,
        _task: &'task Task,
        _now: Instant,
    ) -> Self::TransportImpl<'task>
    where
        Task: SwapTask,
    {
        SwapClient::default()
    }
}
