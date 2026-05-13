use serde::Serialize;

use platform::{batch::Batch, result::Result as PlatformResult};
use sdk::cosmwasm_std::Addr;

use crate::msg::{
    CloseLeaseParams, LeaseOperationsMsg, OpenLeaseParams, SwapParams, TransferOutParams,
};

/// Marker trait the consuming controller must implement on its own
/// `ExecuteMsg` (or whichever outer enum carries [`LeaseOperationsMsg`]).
///
/// Why a marker: the stub builders accept an `op_to_msg` closure that wraps
/// a [`LeaseOperationsMsg`] into the controller's outer message before it
/// goes onto the wire. Without this bound, any `Serialize` value would work
/// — including `LeaseOperationsMsg` itself, which would let the controller
/// (or a contributor) accidentally emit a flat-embedded operation that
/// bypasses the controller's own authorisation layer. By requiring the
/// closure's output to implement `ControllerInnerMessage` the crate forces
/// a deliberate one-line opt-in on the consumer side; the orphan rule
/// prevents the consumer from implementing it on `LeaseOperationsMsg`.
pub trait ControllerInnerMessage: Serialize {}

pub struct Factory<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Factory<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn open<F, M>(&self, params: OpenLeaseParams, op_to_msg: F) -> PlatformResult<Batch>
    where
        F: FnOnce(LeaseOperationsMsg) -> M,
        M: ControllerInnerMessage,
    {
        schedule(
            self.controller,
            &op_to_msg(LeaseOperationsMsg::OpenLease(params)),
        )
    }

    pub fn close<F, M>(&self, params: CloseLeaseParams, op_to_msg: F) -> PlatformResult<Batch>
    where
        F: FnOnce(LeaseOperationsMsg) -> M,
        M: ControllerInnerMessage,
    {
        schedule(
            self.controller,
            &op_to_msg(LeaseOperationsMsg::CloseLease(params)),
        )
    }
}

pub struct Lease<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Lease<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn swap<F, M>(&self, params: SwapParams, op_to_msg: F) -> PlatformResult<Batch>
    where
        F: FnOnce(LeaseOperationsMsg) -> M,
        M: ControllerInnerMessage,
    {
        schedule(
            self.controller,
            &op_to_msg(LeaseOperationsMsg::Swap(params)),
        )
    }

    pub fn transfer_out<F, M>(
        &self,
        params: TransferOutParams,
        op_to_msg: F,
    ) -> PlatformResult<Batch>
    where
        F: FnOnce(LeaseOperationsMsg) -> M,
        M: ControllerInnerMessage,
    {
        schedule(
            self.controller,
            &op_to_msg(LeaseOperationsMsg::TransferOut(params)),
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
    use serde::Serialize;

    use currencies::{
        PaymentGroup,
        testing::{PaymentC1, PaymentC2, PaymentC3},
    };
    use finance::coin::Coin;
    use sdk::cosmwasm_std::Addr;

    use crate::msg::{
        CloseLeaseParams, LeaseOperationsMsg, OpenLeaseParams, SwapParams, TransferOutParams,
    };

    use super::{ControllerInnerMessage, Factory, Lease};

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum OuterExecuteMsg {
        LeaseOperations(LeaseOperationsMsg),
    }

    impl ControllerInnerMessage for OuterExecuteMsg {}

    #[test]
    fn factory_open_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = Factory::new(&controller);
        let batch = factory
            .open(sample_open_lease_params(), OuterExecuteMsg::LeaseOperations)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn factory_close_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = Factory::new(&controller);
        let batch = factory
            .close(CloseLeaseParams {}, OuterExecuteMsg::LeaseOperations)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_swap_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = Lease::new(&controller);
        let batch = lease
            .swap(sample_swap_params(), OuterExecuteMsg::LeaseOperations)
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn lease_transfer_out_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let lease = Lease::new(&controller);
        let batch = lease
            .transfer_out(
                sample_transfer_out_params(),
                OuterExecuteMsg::LeaseOperations,
            )
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
        SwapParams::new(
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
