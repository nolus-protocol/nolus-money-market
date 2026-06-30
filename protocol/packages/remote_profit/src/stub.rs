use serde::Serialize;

use finance::duration::Duration;
use platform::{
    batch::{Batch, ReplyId},
    result::Result as PlatformResult,
};
use sdk::cosmwasm_std::Addr;

use crate::msg::{CloseProfitParams, OpenProfitParams, SwapParams, TransferOutParams};

/// Marker trait the consuming controller must implement on its own
/// `ExecuteMsg` (or whichever outer enum carries the per-operation
/// `{ params, timeout }` variants).
///
/// Why a marker: the stub builders accept a `msg` closure that wraps the
/// typed `*Params` and the `Duration` into the controller's outer message
/// before it goes onto the wire. Without this bound, any `Serialize` value
/// would work — including raw `*Params` — which would let the controller
/// (or a contributor) accidentally emit a flat-embedded params payload that
/// bypasses the controller's own authorisation layer. By requiring the
/// closure's output to implement `ControllerInnerMessage` the crate forces
/// a deliberate one-line opt-in on the consumer side; the orphan rule
/// prevents the consumer from implementing it on the `*Params` types.
pub trait ControllerInnerMessage: Serialize {}

pub struct Factory<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Factory<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn open<F, M>(
        &self,
        params: OpenProfitParams,
        timeout: Duration,
        msg: F,
    ) -> PlatformResult<Batch>
    where
        F: FnOnce(OpenProfitParams, Duration) -> M,
        M: ControllerInnerMessage,
    {
        schedule(self.controller, &msg(params, timeout))
    }
}

pub struct Profit<'controller> {
    controller: &'controller Addr,
}

impl<'controller> Profit<'controller> {
    pub const fn new(controller: &'controller Addr) -> Self {
        Self { controller }
    }

    pub fn swap<F, M>(&self, params: SwapParams, timeout: Duration, msg: F) -> PlatformResult<Batch>
    where
        F: FnOnce(SwapParams, Duration) -> M,
        M: ControllerInnerMessage,
    {
        schedule(self.controller, &msg(params, timeout))
    }

    pub fn transfer_out<F, M>(
        &self,
        params: TransferOutParams,
        timeout: Duration,
        msg: F,
    ) -> PlatformResult<Batch>
    where
        F: FnOnce(TransferOutParams, Duration) -> M,
        M: ControllerInnerMessage,
    {
        schedule(self.controller, &msg(params, timeout))
    }

    /// Schedule a `CloseProfit` as a reply-on-error sub-message
    ///
    /// The close is best-effort by design: it rides the same transaction
    /// as the customer payout, and a synchronous controller failure (for
    /// example, a non-operational channel) must not revert that payout.
    /// The caller absorbs the failure in its `reply` handler instead.
    pub fn close<F, M>(
        &self,
        params: CloseProfitParams,
        timeout: Duration,
        reply_id: ReplyId,
        msg: F,
    ) -> PlatformResult<Batch>
    where
        F: FnOnce(CloseProfitParams, Duration) -> M,
        M: ControllerInnerMessage,
    {
        schedule_reply_on_error(self.controller, &msg(params, timeout), reply_id)
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

fn schedule_reply_on_error<M>(
    controller: &Addr,
    msg: &M,
    reply_id: ReplyId,
) -> PlatformResult<Batch>
where
    M: Serialize + ?Sized,
{
    let mut batch = Batch::default();
    batch
        .schedule_execute_wasm_reply_on_error_no_funds(controller.clone(), msg, reply_id)
        .map(|()| batch)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use currencies::testing::{PaymentC1, PaymentC2, PaymentC3};
    use finance::{coin::Coin, duration::Duration};
    use sdk::cosmwasm_std::Addr;

    use crate::msg::{CloseProfitParams, OpenProfitParams, SwapParams, TransferOutParams};

    use super::{ControllerInnerMessage, Factory, Profit};

    /// Mirrors the production controller's `ExecuteMsg` per-variant struct
    /// shape (`protocol/contracts/remote_profit/src/api.rs`).
    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum OuterExecuteMsg {
        OpenProfit {
            params: OpenProfitParams,
            timeout: Duration,
        },
        CloseProfit {
            params: CloseProfitParams,
            timeout: Duration,
        },
        Swap {
            params: SwapParams,
            timeout: Duration,
        },
        TransferOut {
            params: TransferOutParams,
            timeout: Duration,
        },
    }

    impl ControllerInnerMessage for OuterExecuteMsg {}

    #[test]
    fn factory_open_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let factory = Factory::new(&controller);
        let batch = factory
            .open(
                sample_open_profit_params(),
                OpenProfitParams::TIMEOUT,
                |params, timeout| OuterExecuteMsg::OpenProfit { params, timeout },
            )
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn profit_close_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let profit = Profit::new(&controller);
        let batch = profit
            .close(
                CloseProfitParams {},
                CloseProfitParams::TIMEOUT,
                1,
                |params, timeout| OuterExecuteMsg::CloseProfit { params, timeout },
            )
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn profit_swap_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let profit = Profit::new(&controller);
        let batch = profit
            .swap(
                sample_swap_params(),
                SwapParams::TIMEOUT,
                |params, timeout| OuterExecuteMsg::Swap { params, timeout },
            )
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn profit_transfer_out_schedules_one_message() {
        let controller = Addr::unchecked("controller");
        let profit = Profit::new(&controller);
        let batch = profit
            .transfer_out(
                sample_transfer_out_params(),
                TransferOutParams::TIMEOUT,
                |params, timeout| OuterExecuteMsg::TransferOut { params, timeout },
            )
            .expect("scheduling must succeed");
        assert_eq!(1, batch.len());
    }

    fn sample_open_profit_params() -> OpenProfitParams {
        OpenProfitParams::new(7)
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
