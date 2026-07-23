use serde::{Deserialize, Serialize};

use currencies::{LeaseGroup, Lpns};
use dex::{RemoteLeaseTransport, RemoteLeaseTransportFactory, SwapError, SwapResult, SwapTask};
use finance::instant::Instant;
use platform::batch::Batch;
use remote_lease::{stub::Lease, swap::SwapParams};
use sdk::cosmwasm_std::Addr;

use crate::api::LeasePaymentCurrencies;

#[derive(Default, Serialize, Deserialize)]
pub struct SwapClientFactory {}

impl RemoteLeaseTransportFactory for SwapClientFactory {
    type TopG = LeasePaymentCurrencies;
    type TransportImpl<'task> = SwapClientTransport<'task>;

    fn transport<'task, Task>(&self, task: &'task Task, _now: Instant) -> Self::TransportImpl<'task>
    where
        Task: SwapTask,
    {
        SwapClientTransport::new(task.dex_account().remote_controller())
    }
}

pub struct SwapClientTransport<'controller> {
    controller: &'controller Addr,
}

impl<'controller> SwapClientTransport<'controller> {
    fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }
}

impl RemoteLeaseTransport<LeasePaymentCurrencies> for SwapClientTransport<'_> {
    fn swap(
        self,
        params: SwapParams<LeasePaymentCurrencies, LeasePaymentCurrencies>,
    ) -> SwapResult<Batch> {
        Lease::<LeaseGroup, Lpns, LeasePaymentCurrencies>::new(self.controller)
            .swap(
                params,
                SwapParams::<LeasePaymentCurrencies, LeasePaymentCurrencies>::TIMEOUT,
            )
            .map_err(SwapError::from)
    }
}
