use std::iter;

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use cw_time::IntoInstant;
use dex::{DrainStage, Enterable, Error as DexError, RemoteTransferOutTask};
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::batch::Batch;
use remote_lease::{
    msg::TransferOutParams,
    response::OperationResponse,
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        PaymentCoin,
        query::{
            StateResponse as QueryStateResponse,
            opened::{OngoingTrx, PositionCloseTrx, RepayTrx, Status},
        },
    },
    contract::{
        Lease,
        state::{Response, State, SwapResult, arrival, opened},
    },
    error::{ContractError, ContractResult},
    event::Type,
    finance::{LpnCoinDTO, LpnCurrencies},
};

use super::{close::Closable, payment::Repayable, repay};

/// A non-`TransferOut` success acknowledgment can only come from a buggy
/// or hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_TRANSFER_OUT_RESPONSE: &str = "non-transfer-out operation response";

type ProceedsGroup = LpnCurrencies;

/// The home-bound drain of a swap's LPN proceeds
///
/// Generic over the finisher that decides what to do with the proceeds
/// once they have arrived on the local account, so every swap leg that
/// drains LPN home reuses the same transfer-out wire and stage mapping.
#[derive(Serialize, Deserialize)]
pub(crate) struct RepayDrain<Finisher> {
    lease: Lease,
    proceeds: LpnCoinDTO,
    finisher: Finisher,
    /// The local-account balance in the proceeds' currency at the moment the
    /// drain was entered — before any coin had been drained back. The arrival
    /// check measures against this baseline, never an absolute balance, so a
    /// balance the lease already held cannot be mistaken for the proceeds
    /// arriving. Persisted with the task so it survives every callback's serde
    /// round-trip; captured once at construction and never recomputed.
    baseline: Vec<LpnCoinDTO>,
}

impl<Finisher> RepayDrain<Finisher> {
    pub(in super::super) fn new(
        lease: Lease,
        proceeds: LpnCoinDTO,
        finisher: Finisher,
        account: &Addr,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Self> {
        arrival::snapshot_baseline(&[proceeds], account, querier)
            .map_err(ContractError::from)
            .map(|baseline| Self {
                lease,
                proceeds,
                finisher,
                baseline,
            })
    }
}

impl<Finisher> RepayDrain<Finisher>
where
    Finisher: ProceedsFinish,
    dex::StateDrain<RepayDrain<Finisher>>: Into<State>,
{
    pub(in super::super) fn start(self, env: &Env, querier: QuerierWrapper<'_>) -> SwapResult {
        dex::start_drain(self)
            .and_then(|start_drain| enter_drain(start_drain, env, querier))
            .map_err(Into::into)
    }
}

impl<Finisher> RemoteTransferOutTask for RepayDrain<Finisher>
where
    Finisher: ProceedsFinish,
{
    type G = ProceedsGroup;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::RepaymentTransferOut
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    /// Authorised against the controller pinned in `LeaseDTO` — the swap
    /// phase authorised the same controller's callback, so a leaser
    /// re-configuration can neither wedge nor hijack the proceeds drain.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(
            &SingleUserPermission::new(&self.lease.lease.remote_lease_controller),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::G>> {
        iter::once(self.proceeds)
    }

    fn schedule_transfer_out(&self, coin: &CoinDTO<Self::G>, nonce: u64) -> dex::DexResult<Batch> {
        transfer_out_msg(&self.lease.lease.remote_lease_controller, coin, nonce)
    }

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<()> {
        decode_response(payload)
    }

    fn all_received(&self, account: &Addr, querier: QuerierWrapper<'_>) -> dex::DexResult<bool> {
        arrival::arrived_over_baseline(&[self.proceeds], &self.baseline, account, querier)
    }

    fn finish(self, env: &Env, querier: QuerierWrapper<'_>) -> Self::Result {
        self.finisher
            .finish(self.lease, self.proceeds, env, querier)
    }

    fn state(
        self,
        in_progress: DrainStage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.finisher
            .state(self.lease, in_progress, now, due_projection, querier)
    }
}

/// What to do with a swap's LPN proceeds after they reach the local account
///
/// Owns both the terminal transition and the in-flight query mapping so a
/// single drain spec serves every swap leg without per-leg `enum` arms.
pub(crate) trait ProceedsFinish {
    fn finish(
        self,
        lease: Lease,
        proceeds: LpnCoinDTO,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> SwapResult;

    fn state(
        self,
        lease: Lease,
        in_progress: DrainStage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RepayFinish {
    payment: PaymentCoin,
}

impl RepayFinish {
    pub(in super::super) fn new(payment: PaymentCoin) -> Self {
        Self { payment }
    }
}

impl ProceedsFinish for RepayFinish {
    fn finish(
        self,
        lease: Lease,
        proceeds: LpnCoinDTO,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> SwapResult {
        repay::repay(lease, proceeds, env, querier)
    }

    fn state(
        self,
        lease: Lease,
        in_progress: DrainStage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        let in_progress = match in_progress {
            DrainStage::TransferOut { acks_left: _ } => RepayTrx::TransferOut,
            DrainStage::FundsArrival => RepayTrx::TransferInFinish,
        };
        let in_progress = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress,
        };

        opened::lease_state(
            lease,
            Status::InProgress(in_progress),
            now,
            due_projection,
            querier,
        )
    }
}

/// Resume a position-close swap leg once its LPN proceeds arrive home
///
/// Owns the `Repayable` moved out of `SellAsset::finish`, so the same drain
/// spec serves every liquidation and customer-close flavour. Both drain
/// stages map onto `PositionCloseTrx::TransferInFinish` — the swap and any
/// asset-direction transfer already happened on the remote side, so the
/// only close transaction left visible is the inbound proceeds settlement.
#[derive(Serialize, Deserialize)]
pub(crate) struct CloseFinish<R> {
    repayable: R,
}

impl<R> CloseFinish<R> {
    pub(in super::super) fn new(repayable: R) -> Self {
        Self { repayable }
    }
}

impl<R> ProceedsFinish for CloseFinish<R>
where
    R: Repayable + Closable,
{
    fn finish(
        self,
        lease: Lease,
        proceeds: LpnCoinDTO,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> SwapResult {
        self.repayable.try_repay(lease, proceeds, env, querier)
    }

    fn state(
        self,
        lease: Lease,
        _in_progress: DrainStage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        let trx = self
            .repayable
            .transaction(&lease, PositionCloseTrx::TransferInFinish);

        opened::lease_state(lease, Status::InProgress(trx), now, due_projection, querier)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
        nonce: u64,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

fn enter_drain<Finisher>(
    start_drain: dex::StartDrainState<RepayDrain<Finisher>>,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> dex::DexResult<Response>
where
    Finisher: ProceedsFinish,
    dex::StateDrain<RepayDrain<Finisher>>: Into<State>,
{
    start_drain
        .enter(env.block.time.into_instant(), querier)
        .map(|drain_msgs| Response::from(drain_msgs, dex::StateDrain::from(start_drain)))
}

fn transfer_out_msg(
    controller: &Addr,
    coin: &CoinDTO<ProceedsGroup>,
    nonce: u64,
) -> dex::DexResult<Batch> {
    TransferOutParams::new(coin.into_super_group())
        .map_err(DexError::remote_swap_client)
        .and_then(|params| {
            ControllerLease::new(controller)
                .transfer_out(params, TransferOutParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::TransferOut {
                        params,
                        timeout,
                        nonce,
                    }
                })
                .map_err(Into::into)
        })
}

fn decode_response(payload: &[u8]) -> dex::DexResult<()> {
    cosmwasm_std::from_json::<OperationResponse>(payload)
        .map_err(DexError::remote_swap_client)
        .and_then(|response| match response {
            OperationResponse::TransferOut(_confirmation) => Ok(()),
            OperationResponse::OpenLease(_)
            | OperationResponse::CloseLease(_)
            | OperationResponse::Swap(_) => Err(DexError::unexpected_response_variant(
                NON_TRANSFER_OUT_RESPONSE,
            )),
        })
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::testing::LeaseC2;
    use currency::CurrencyDef;
    use finance::coin::{Coin, CoinDTO};
    use remote_lease::{
        msg::TransferOutParams,
        response::{
            CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId, SwapResponse,
            TransferOutResponse,
        },
    };
    use sdk::cosmwasm_std::{
        self, Addr, Coin as CwCoin, Empty, QuerierWrapper, testing::MockQuerier,
    };

    use crate::finance::{LpnCurrencies, LpnCurrency};

    const CONTROLLER: &str = "controller";
    const LEASE: &str = "lease";

    #[test]
    fn transfer_out_msg_shape_matches_the_controller_wire() {
        const NONCE: u64 = 7;
        let proceeds: CoinDTO<LpnCurrencies> = Coin::<LpnCurrency>::new(1000).into();
        let params =
            TransferOutParams::new(proceeds.into_super_group()).expect("a non-zero amount");
        let msg = super::ControllerExecuteMsg::TransferOut {
            params: params.clone(),
            timeout: TransferOutParams::TIMEOUT,
            nonce: NONCE,
        };

        let expected = format!(
            r#"{{"transfer_out":{{"params":{},"timeout":{},"nonce":{}}}}}"#,
            cosmwasm_std::to_json_string(&params).expect("the params should serialize"),
            cosmwasm_std::to_json_string(&TransferOutParams::TIMEOUT)
                .expect("the timeout should serialize"),
            NONCE,
        );
        assert_eq!(
            expected,
            cosmwasm_std::to_json_string(&msg).expect("the message should serialize")
        );
    }

    #[test]
    fn transfer_out_msg_targets_the_controller() {
        let coin: CoinDTO<LpnCurrencies> = Coin::<LpnCurrency>::new(1000).into();
        let batch = super::transfer_out_msg(&Addr::unchecked(CONTROLLER), &coin, 7)
            .expect("a valid transfer-out message");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn decode_accepts_a_transfer_out_response() {
        let payload = OperationResponse::TransferOut(TransferOutResponse {});

        assert_eq!(Ok(()), decode(&payload).map_err(|err| err.to_string()));
    }

    #[test]
    fn decode_rejects_non_transfer_out_responses() {
        let amount_out: CoinDTO<currencies::PaymentGroup> = Coin::<LeaseC2>::new(1000).into();
        let unexpected = [
            OperationResponse::OpenLease(OpenLeaseResponse {
                remote_lease_id: remote_lease_id(),
            }),
            OperationResponse::CloseLease(CloseLeaseResponse {}),
            OperationResponse::Swap(SwapResponse { amount_out }),
        ];
        unexpected.into_iter().for_each(|payload| {
            assert!(matches!(
                decode(&payload),
                Err(dex::Error::UnexpectedResponseVariant(_reason))
            ));
        });
    }

    /// The proceeds have arrived only once the balance rises over its entry
    /// baseline by the full proceeds amount: a pre-existing balance equal to
    /// the proceeds is not mistaken for an arrival, while the zero-baseline
    /// case is unchanged from an absolute balance check.
    #[test]
    fn proceeds_arrival_measured_over_baseline() {
        const PROCEEDS: u128 = 1_000;
        let expected: CoinDTO<LpnCurrencies> = Coin::<LpnCurrency>::new(PROCEEDS).into();

        assert_eq!(Ok(false), arrived(&expected, 0, PROCEEDS - 1));
        assert_eq!(Ok(true), arrived(&expected, 0, PROCEEDS));

        assert_eq!(Ok(false), arrived(&expected, PROCEEDS, PROCEEDS));
        assert_eq!(Ok(true), arrived(&expected, PROCEEDS, PROCEEDS + PROCEEDS));
    }

    fn decode(payload: &OperationResponse) -> dex::DexResult<()> {
        cosmwasm_std::to_json_vec(payload)
            .map_err(dex::Error::remote_swap_client)
            .and_then(|payload| super::decode_response(&payload))
    }

    fn arrived(
        expected: &CoinDTO<LpnCurrencies>,
        baseline_balance: u128,
        arrival_balance: u128,
    ) -> Result<bool, String> {
        let account = Addr::unchecked(LEASE);
        super::arrival::snapshot_baseline(
            &[*expected],
            &account,
            QuerierWrapper::new(&held(baseline_balance)),
        )
        .map_err(|err| err.to_string())
        .and_then(|baseline| {
            super::arrival::arrived_over_baseline(
                &[*expected],
                &baseline,
                &account,
                QuerierWrapper::new(&held(arrival_balance)),
            )
            .map_err(|err| err.to_string())
        })
    }

    fn held(balance: u128) -> MockQuerier<Empty> {
        MockQuerier::<Empty>::new(&[(
            LEASE,
            &[CwCoin::new(
                balance,
                LpnCurrency::dto().definition().bank_symbol,
            )],
        )])
    }

    fn remote_lease_id() -> RemoteLeaseId {
        RemoteLeaseId::new(String::from("StubPda11111111111111111111111111111"))
            .expect("a base58 sample")
    }
}
