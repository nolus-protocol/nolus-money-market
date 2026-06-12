use std::iter;

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use currency::CurrencyDef;
use dex::{DrainStage, Error as DexError, RemoteTransferOutTask};
use finance::{
    coin::{Coin, CoinDTO, WithCoin},
    duration::Duration,
    instant::Instant,
};
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
        state::{SwapResult, paid::remote_close::ClosingRemoteLease},
    },
    error::ContractResult,
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

pub(super) fn start(lease: Lease) -> ContractResult<StartState> {
    dex::start_drain(TransferOut::new(lease)).map_err(Into::into)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TransferOut {
    lease: Lease,
}

impl TransferOut {
    fn new(lease: Lease) -> Self {
        Self { lease }
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
        received(self.amount(), account, querier)
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

fn received(
    expected: &CoinDTO<AssetGroup>,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> dex::DexResult<bool> {
    struct CheckBalance<'account, 'querier> {
        account: &'account Addr,
        querier: QuerierWrapper<'querier>,
    }

    impl WithCoin<AssetGroup> for CheckBalance<'_, '_> {
        type Outcome = dex::DexResult<bool>;

        fn on<C>(self, expected: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
        {
            bank::balance::<C>(self.account, self.querier)
                .map_err(Into::into)
                .map(|ref balance| expected <= *balance)
        }
    }

    expected.with_coin(CheckBalance { account, querier })
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

    #[test]
    fn received_only_when_balance_covers_the_amount() {
        let expected: CoinDTO<LeaseAssetCurrencies> = Coin::<LeaseC2>::new(1000).into();
        let account = Addr::unchecked(LEASE);

        assert_eq!(Ok(false), query_received(&expected, &account, 999));
        assert_eq!(Ok(true), query_received(&expected, &account, 1000));
        assert_eq!(Ok(true), query_received(&expected, &account, 1500));
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

    fn query_received(
        expected: &CoinDTO<LeaseAssetCurrencies>,
        account: &Addr,
        balance: u128,
    ) -> Result<bool, String> {
        let mock_querier = MockQuerier::<Empty>::new(&[(
            LEASE,
            &[CwCoin::new(
                balance,
                LeaseC2::dto().definition().bank_symbol,
            )],
        )]);
        super::received(expected, account, QuerierWrapper::new(&mock_querier))
            .map_err(|err| err.to_string())
    }

    fn remote_lease_id() -> RemoteLeaseId {
        RemoteLeaseId::new(String::from("StubPda11111111111111111111111111111"))
            .expect("a base58 sample")
    }
}
