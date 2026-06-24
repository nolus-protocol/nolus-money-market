use std::iter;

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use dex::{DrainStage, Error as DexError, RemoteTransferOutTask};
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::{
    bank,
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::{
    msg::TransferOutParams,
    response::OperationResponse,
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeaseAssetCurrencies,
        query::{StateResponse as QueryStateResponse, paid::ClosingTrx},
    },
    contract::{
        Lease,
        cmd::Close,
        state::{SwapResult, arrival, paid::remote_close::ClosingRemoteLease},
    },
    error::{ContractError, ContractResult},
    event::Type,
    lease::{LeaseDTO, with_lease_paid},
};

/// A non-`TransferOut` success acknowledgment can only come from a buggy
/// or hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_TRANSFER_OUT_RESPONSE: &str = "non-transfer-out operation response";

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState = dex::StartDrainState<TransferOut>;
pub(in super::super) type DexState = dex::StateDrain<TransferOut>;

pub(super) fn start(
    lease: Lease,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> ContractResult<StartState> {
    TransferOut::enter(lease, account, querier)
        .and_then(|task| dex::start_drain(task).map_err(Into::into))
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TransferOut {
    lease: Lease,
    /// The local-account balance in the lease asset's currency at the moment
    /// the close drain was entered — before the asset had been drained back.
    /// The arrival check measures against this baseline, never an absolute
    /// balance, so a balance the lease already held cannot be mistaken for the
    /// asset arriving. Persisted with the task so it survives every callback's
    /// serde round-trip; captured once at construction and never recomputed.
    baseline: Vec<CoinDTO<AssetGroup>>,
}

impl TransferOut {
    fn enter(lease: Lease, account: &Addr, querier: QuerierWrapper<'_>) -> ContractResult<Self> {
        arrival::snapshot_baseline(&[*lease.lease.position.amount()], account, querier)
            .map_err(ContractError::from)
            .map(|baseline| Self { lease, baseline })
    }

    fn amount(&self) -> &CoinDTO<AssetGroup> {
        self.lease.lease.position.amount()
    }

    fn emit_closed(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl RemoteTransferOutTask for TransferOut {
    type G = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::ClosingTransferOut
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    /// Authorised against the controller pinned in `LeaseDTO` — the same
    /// address `schedule_transfer_out` emits to and the `Closed` terminal
    /// authorises against — so a leaser re-configuration can neither
    /// wedge nor hijack a live drain.
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
        iter::once(*self.amount())
    }

    fn schedule_transfer_out(&self, coin: &CoinDTO<Self::G>) -> dex::DexResult<Batch> {
        transfer_out_msg(&self.lease.lease.remote_lease_controller, coin)
    }

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<()> {
        decode_response(payload)
    }

    fn all_received(&self, account: &Addr, querier: QuerierWrapper<'_>) -> dex::DexResult<bool> {
        arrival::arrived_over_baseline(&[*self.amount()], &self.baseline, account, querier)
    }

    fn finish(self, env: &Env, querier: QuerierWrapper<'_>) -> Self::Result {
        let lease_addr = self.lease.lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        let emitter = self.emit_closed(env, &self.lease.lease);
        let customer = self.lease.lease.customer.clone();
        let closing = ClosingRemoteLease::from(&self.lease.lease);

        closing
            .schedule_close()
            .and_then(|close_lease_msgs| {
                with_lease_paid::execute(self.lease.lease, Close::new(lease_account)).map(
                    |payout_msgs| payout_msgs.merge(close_lease_msgs), //the payout must precede the best-effort CloseLease
                )
            })
            .and_then(|close_msgs| {
                self.lease
                    .leases
                    .finalize_lease(customer)
                    .map(|finalizer_msgs| close_msgs.merge(finalizer_msgs)) //make sure the finalizer messages go out last
            })
            .map(|all_messages| MessageResponse::messages_with_event(all_messages, emitter))
            .map(|response| StateMachineResponse::from(response, closing))
    }

    fn state(
        self,
        in_progress: DrainStage,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        let in_progress = match in_progress {
            DrainStage::TransferOut { acks_left: _ } => ClosingTrx::TransferOut,
            DrainStage::FundsArrival => ClosingTrx::TransferInFinish,
        };
        Ok(QueryStateResponse::closing_from(
            self.lease.lease,
            in_progress,
        ))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

fn transfer_out_msg(controller: &Addr, coin: &CoinDTO<AssetGroup>) -> dex::DexResult<Batch> {
    TransferOutParams::new(coin.into_super_group())
        .map_err(DexError::remote_swap_client)
        .and_then(|params| {
            ControllerLease::new(controller)
                .transfer_out(params, TransferOutParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::TransferOut { params, timeout }
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

    use crate::api::{LeaseAssetCurrencies, LeasePaymentCurrencies};

    const CONTROLLER: &str = "controller";
    const LEASE: &str = "lease";

    #[test]
    fn transfer_out_msg_shape_matches_the_controller_wire() {
        let amount: CoinDTO<LeasePaymentCurrencies> = Coin::<LeaseC2>::new(1000).into();
        let params = TransferOutParams::new(amount).expect("a non-zero amount");
        let msg = super::ControllerExecuteMsg::TransferOut {
            params: params.clone(),
            timeout: TransferOutParams::TIMEOUT,
        };

        let expected = format!(
            r#"{{"transfer_out":{{"params":{},"timeout":{}}}}}"#,
            cosmwasm_std::to_json_string(&params).expect("the params should serialize"),
            cosmwasm_std::to_json_string(&TransferOutParams::TIMEOUT)
                .expect("the timeout should serialize"),
        );
        assert_eq!(
            expected,
            cosmwasm_std::to_json_string(&msg).expect("the message should serialize")
        );
    }

    #[test]
    fn decode_accepts_a_transfer_out_response() {
        let payload = OperationResponse::TransferOut(TransferOutResponse {});

        assert_eq!(Ok(()), decode(&payload).map_err(|err| err.to_string()));
    }

    #[test]
    fn decode_rejects_non_transfer_out_responses() {
        let amount_out: CoinDTO<LeasePaymentCurrencies> = Coin::<LeaseC2>::new(1000).into();
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

    /// The asset has arrived only once the balance rises over its entry
    /// baseline by the full position amount: a pre-existing balance equal to
    /// the amount is not mistaken for an arrival, while the zero-baseline case
    /// is unchanged from an absolute balance check.
    #[test]
    fn asset_arrival_measured_over_baseline() {
        const AMOUNT: u128 = 1_000;
        let expected: CoinDTO<LeaseAssetCurrencies> = Coin::<LeaseC2>::new(AMOUNT).into();

        assert_eq!(Ok(false), arrived(&expected, 0, AMOUNT - 1));
        assert_eq!(Ok(true), arrived(&expected, 0, AMOUNT));

        assert_eq!(Ok(false), arrived(&expected, AMOUNT, AMOUNT));
        assert_eq!(Ok(true), arrived(&expected, AMOUNT, AMOUNT + AMOUNT));
    }

    #[test]
    fn transfer_out_msg_targets_the_controller() {
        let coin: CoinDTO<LeaseAssetCurrencies> = Coin::<LeaseC2>::new(1000).into();
        let batch = super::transfer_out_msg(&Addr::unchecked(CONTROLLER), &coin)
            .expect("a valid transfer-out message");
        assert_eq!(1, batch.len());
    }

    fn decode(payload: &OperationResponse) -> dex::DexResult<()> {
        cosmwasm_std::to_json_vec(payload)
            .map_err(dex::Error::remote_swap_client)
            .and_then(|payload| super::decode_response(&payload))
    }

    fn arrived(
        expected: &CoinDTO<LeaseAssetCurrencies>,
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
                LeaseC2::dto().definition().bank_symbol,
            )],
        )])
    }

    fn remote_lease_id() -> RemoteLeaseId {
        RemoteLeaseId::new(String::from("StubPda11111111111111111111111111111"))
            .expect("a base58 sample")
    }
}
