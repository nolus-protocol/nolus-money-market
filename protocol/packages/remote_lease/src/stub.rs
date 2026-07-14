use serde::Serialize;

use finance::duration::Duration;
use platform::{batch::Batch, result::Result as PlatformResult};
use sdk::cosmwasm_std::Addr;

use crate::msg::{CloseLeaseParams, ExecuteMsg, OpenLeaseParams, SwapParams, TransferOutParams};

pub struct Factory<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Factory<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn open(&self, params: OpenLeaseParams, timeout: Duration) -> PlatformResult<Batch> {
        schedule(self.controller, &ExecuteMsg::OpenLease { params, timeout })
    }

    pub fn close(&self, params: CloseLeaseParams, timeout: Duration) -> PlatformResult<Batch> {
        schedule(self.controller, &ExecuteMsg::CloseLease { params, timeout })
    }
}

pub struct Lease<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Lease<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn swap(&self, params: SwapParams, timeout: Duration) -> PlatformResult<Batch> {
        schedule(self.controller, &ExecuteMsg::Swap { params, timeout })
    }

    pub fn transfer_out(
        &self,
        params: TransferOutParams,
        timeout: Duration,
    ) -> PlatformResult<Batch> {
        schedule(
            self.controller,
            &ExecuteMsg::TransferOut { params, timeout },
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

    #[test]
    fn factory_open_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = Factory::new(&controller);
        let batch = factory
            .open(sample_open_lease_params(), OpenLeaseParams::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn factory_close_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = Factory::new(&controller);
        let batch = factory
            .close(CloseLeaseParams {}, CloseLeaseParams::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_swap_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = Lease::new(&controller);
        let batch = lease
            .swap(sample_swap_params(), SwapParams::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_transfer_out_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = Lease::new(&controller);
        let batch = lease
            .transfer_out(sample_transfer_out_params(), TransferOutParams::TIMEOUT)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    fn sample_open_lease_params() -> OpenLeaseParams {
        OpenLeaseParams::new(
            7,
            currency::dto::<PaymentC1, PaymentGroup>(),
            currency::dto::<PaymentC2, PaymentGroup>(),
            currency::dto::<PaymentC3, PaymentGroup>(),
        )
        .expect("sample uses three distinct currencies")
    }

    fn sample_swap_params() -> SwapParams {
        SwapParams::one(
            Coin::<PaymentC1>::new(1000).into(),
            Coin::<PaymentC2>::new(42).into(),
        )
        .expect("sample uses two distinct non-zero amounts")
    }

    fn sample_transfer_out_params() -> TransferOutParams {
        TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
            .expect("sample uses a non-zero amount")
    }
}
