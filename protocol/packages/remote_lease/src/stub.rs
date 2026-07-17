use std::marker::PhantomData;

use currency::Group;
use serde::Serialize;

use finance::duration::Duration;
use platform::{batch::Batch, result::Result as PlatformResult};
use sdk::cosmwasm_std::Addr;

use crate::msg::{CloseLeaseParams, ExecuteMsg, OpenLeaseParams, SwapParams, TransferOutParams};

/// Builds outbound `OpenLease` / `CloseLease` batches addressed to the
/// `remote_lease` controller.
///
/// Generic over the lease's asset (`LeaseG`), LPN (`LpnG`), and payment
/// (`PaymentG`) currency groups; see [`OpenLeaseParams`].
pub struct Factory<'controller, LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    controller: &'controller Addr,
    _g_lease: PhantomData<LeaseG>,
    _g_lpn: PhantomData<LpnG>,
    _g_payment: PhantomData<PaymentG>,
}

impl<'controller, LeaseG, LpnG, PaymentG> Factory<'controller, LeaseG, LpnG, PaymentG>
where
    LeaseG: Group + Serialize,
    LpnG: Group + Serialize,
    PaymentG: Group + Serialize,
{
    pub const fn new(controller: &'controller Addr) -> Self {
        Self {
            controller,
            _g_lease: PhantomData,
            _g_lpn: PhantomData,
            _g_payment: PhantomData,
        }
    }

    pub fn open(
        &self,
        params: OpenLeaseParams<LeaseG, LpnG, PaymentG>,
        timeout: Duration,
    ) -> PlatformResult<Batch> {
        schedule(self.controller, &ExecuteMsg::OpenLease { params, timeout })
    }

    pub fn close(&self, params: CloseLeaseParams, timeout: Duration) -> PlatformResult<Batch> {
        schedule(
            self.controller,
            &ExecuteMsg::<LeaseG, LpnG, PaymentG>::CloseLease { params, timeout },
        )
    }
}

/// Builds outbound `Swap` / `TransferOut` batches addressed to the
/// `remote_lease` controller.
///
/// Generic over the lease's asset (`LeaseG`), LPN (`LpnG`), and payment
/// (`PaymentG`) currency groups; `Swap` and `TransferOut` operate in
/// `PaymentG`.
pub struct Lease<'controller, LeaseG, LpnG, PaymentG>
where
    LeaseG: Group,
    LpnG: Group,
    PaymentG: Group,
{
    controller: &'controller Addr,
    _g_lease: PhantomData<LeaseG>,
    _g_lpn: PhantomData<LpnG>,
    _g_payment: PhantomData<PaymentG>,
}

impl<'controller, LeaseG, LpnG, PaymentG> Lease<'controller, LeaseG, LpnG, PaymentG>
where
    LeaseG: Group + Serialize,
    LpnG: Group + Serialize,
    PaymentG: Group + Serialize,
{
    pub const fn new(controller: &'controller Addr) -> Self {
        Self {
            controller,
            _g_lease: PhantomData,
            _g_lpn: PhantomData,
            _g_payment: PhantomData,
        }
    }

    pub fn swap(
        &self,
        params: SwapParams<PaymentG, PaymentG>,
        timeout: Duration,
    ) -> PlatformResult<Batch> {
        schedule(
            self.controller,
            &ExecuteMsg::<LeaseG, LpnG, PaymentG>::Swap { params, timeout },
        )
    }

    pub fn transfer_out(
        &self,
        params: TransferOutParams<PaymentG>,
        timeout: Duration,
    ) -> PlatformResult<Batch> {
        schedule(
            self.controller,
            &ExecuteMsg::<LeaseG, LpnG, PaymentG>::TransferOut { params, timeout },
        )
    }
}

fn schedule<M>(controller: &Addr, msg: &M) -> PlatformResult<Batch>
where
    M: Serialize + ?Sized,
{
    let mut batch = Batch::default();
    batch
        .schedule_execute_wasm_no_reply_no_funds(controller.clone(), msg)
        .map(|()| batch)
}

#[cfg(test)]
mod tests {
    use currencies::{
        PaymentGroup,
        testing::{PaymentC1, PaymentC2, PaymentC3},
    };
    use finance::coin::Coin;
    use sdk::cosmwasm_std::Addr;

    use crate::msg::{CloseLeaseParams, OpenLeaseParams, SwapParams, TransferOutParams};

    use super::{Factory, Lease};

    type FactoryP2P<'controller> = Factory<'controller, PaymentGroup, PaymentGroup, PaymentGroup>;
    type LeaseP2P<'controller> = Lease<'controller, PaymentGroup, PaymentGroup, PaymentGroup>;
    type TransferOutP2P = TransferOutParams<PaymentGroup>;
    type OpenLeaseP2P = OpenLeaseParams<PaymentGroup, PaymentGroup, PaymentGroup>;

    #[test]
    fn factory_open_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = FactoryP2P::new(&controller);
        let batch = factory
            .open(sample_open_lease_params(), OpenLeaseP2P::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn factory_close_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = FactoryP2P::new(&controller);
        let batch = factory
            .close(CloseLeaseParams {}, CloseLeaseParams::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_swap_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = LeaseP2P::new(&controller);
        let batch = lease
            .swap(
                sample_swap_params(),
                SwapParams::<PaymentGroup, PaymentGroup>::TIMEOUT,
            )
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_transfer_out_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = LeaseP2P::new(&controller);
        let batch = lease
            .transfer_out(sample_transfer_out_params(), TransferOutP2P::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    fn sample_open_lease_params() -> OpenLeaseP2P {
        OpenLeaseParams::new(
            7,
            currency::dto::<PaymentC1, PaymentGroup>(),
            currency::dto::<PaymentC2, PaymentGroup>(),
            currency::dto::<PaymentC3, PaymentGroup>(),
        )
        .expect("sample uses three distinct currencies")
    }

    fn sample_swap_params() -> SwapParams<PaymentGroup, PaymentGroup> {
        SwapParams::one(
            Coin::<PaymentC1>::new(1000).into(),
            Coin::<PaymentC2>::new(42).into(),
        )
        .expect("sample uses two distinct non-zero amounts")
    }

    fn sample_transfer_out_params() -> TransferOutParams<PaymentGroup> {
        TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
            .expect("sample uses a non-zero amount")
    }
}
