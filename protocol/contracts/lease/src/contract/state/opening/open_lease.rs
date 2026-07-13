use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::{CurrencyDef, Group, MemberOf};
use cw_time::IntoInstant;
use finance::{
    coin::{Coin, WithCoin},
    duration::Duration,
    instant::Instant,
    zero::Zero,
};
use lpp::{
    loan::RepayShares,
    stub::loan::{LppLoan, WithLppLoan},
};
use platform::{
    bank::{FixedAddressSender, LazySenderStub},
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    msg::OpenLeaseParams,
    response::{OpenLeaseResponse, OperationResponse, RemoteLeaseId},
    stub::Factory,
};
use reserve::stub::Reserve as _;
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin,
        open::NewLeaseContract,
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        api::Contract,
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{Response, State, open_failed::OpenFailed},
    },
    error::{ContractError, ContractResult},
    finance::{LpnCoin, LpnCurrency, LppRef, OracleRef, ReserveRef},
};

const OPEN_FAILED_EVENT: &str = "ls-remote-lease-open-failed";

/// Open-failure reason recorded when the IBC layer reports the OpenLease
/// packet was never acknowledged.
const TIMEOUT_REASON: &str = "timeout";

/// Open-failure reason recorded when the counterparty acks the OpenLease
/// packet with a success response for a different operation. The offending
/// variant is intentionally not echoed into the reason — it is
/// counterparty-controlled and the controller already logs the raw
/// response; keeping a fixed string avoids interpolating unbounded
/// attacker-influenced data into stored state and events.
const UNEXPECTED_OPERATION_REASON: &str = "unexpected operation response";

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenLease {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
}

impl OpenLease {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Instant,
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
            start_opening_at,
        }
    }

    pub(super) fn enter(&self) -> ContractResult<Batch> {
        OpenLeaseParams::new(
            self.new_lease.expected_instance_ordinal,
            self.downpayment.currency(),
            self.loan.principal.currency().into_super_group(),
            self.new_lease.form.currency.into_super_group(),
        )
        .map_err(ContractError::OpenLeaseParams)
        .and_then(|params| {
            Factory::new(&self.new_lease.remote_lease_controller)
                .open(params, OpenLeaseParams::TIMEOUT)
                .map_err(ContractError::from)
        })
    }

    fn authz_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> ContractResult<()> {
        access_control::check(&self.deps.3.remote_lease_callback_permission(querier), info)
            .map_err(Into::into)
    }

    fn on_open_lease_ack(
        self,
        remote_lease_id: RemoteLeaseId,
        _env: &Env,
    ) -> ContractResult<Response> {
        let next = super::buy_asset::start(
            self.new_lease,
            self.downpayment,
            self.loan,
            self.deps,
            self.start_opening_at,
            remote_lease_id,
        );
        let batch = next.enter();
        Ok(StateMachineResponse::from(
            MessageResponse::messages_only(batch),
            State::from(super::buy_asset::DexState::from(next)),
        ))
    }

    fn on_open_failed(
        self,
        querier: QuerierWrapper<'_>,
        env: &Env,
        reason: RemoteErrorMessage,
    ) -> ContractResult<Response> {
        let lease_addr = env.contract.address.clone();
        let now = env.block.time.into_instant();
        let leases_ref = self.deps.3.clone();
        self.refund_batch(querier, &lease_addr, now)
            .map(|batch| Self::open_failed_response(batch, lease_addr, reason, leases_ref))
    }

    fn refund_batch(
        self,
        querier: QuerierWrapper<'_>,
        lease_addr: &Addr,
        now: Instant,
    ) -> ContractResult<Batch> {
        let customer = self.new_lease.form.customer.clone();
        let downpayment = self.downpayment;
        let leases_ref = self.deps.3.clone();
        self.repay_loan(querier, lease_addr, now)
            .and_then(|reserve_lpp_batch| {
                let customer_batch = downpayment.with_coin(SendToCustomer {
                    customer: customer.clone(),
                });
                leases_ref.finalize_lease(customer).map(|finalize_batch| {
                    reserve_lpp_batch
                        .merge(customer_batch)
                        .merge(finalize_batch)
                })
            })
    }

    fn repay_loan(
        self,
        querier: QuerierWrapper<'_>,
        lease_addr: &Addr,
        now: Instant,
    ) -> Result<Batch, ContractError> {
        Coin::<LpnCurrency>::try_from(self.loan.principal)
            .map_err(ContractError::from)
            .and_then(|principal| {
                ReserveRef::try_new(self.new_lease.form.reserve, &querier)
                    .map_err(ContractError::ReserveError)
                    .and_then(|reserve| {
                        self.deps.0.execute_loan(
                            RepayOpenLoan {
                                principal,
                                now,
                                reserve,
                            },
                            lease_addr.clone(),
                            querier,
                        )
                    })
            })
    }

    fn open_failed_response(
        batch: Batch,
        lease_addr: Addr,
        reason: RemoteErrorMessage,
        leases_ref: LeasesRef,
    ) -> Response {
        let emitter = Emitter::of_type(OPEN_FAILED_EVENT)
            .emit("id", lease_addr)
            .emit("reason", reason.as_str().to_owned());
        StateMachineResponse::from(
            MessageResponse::messages_with_event(batch, emitter),
            State::from(OpenFailed::new(reason, leases_ref)),
        )
    }
}

impl Contract for OpenLease {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        Ok(QueryStateResponse::Opening {
            currency: self.new_lease.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::RequestingOpenLease {},
        })
    }

    fn on_remote_lease_callback(
        self,
        callback: RemoteLeaseCallback,
        info: MessageInfo,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.authz_callback(querier, &info)
            .and_then(|()| match callback {
                RemoteLeaseCallback::OperationOk(OperationResponse::OpenLease(
                    OpenLeaseResponse { remote_lease_id },
                )) => self.on_open_lease_ack(remote_lease_id, &env),
                RemoteLeaseCallback::OperationOk(_unexpected) => {
                    // A success ack for a non-`OpenLease` operation can only
                    // come from a buggy or hostile counterparty. Returning
                    // `Err` here would revert the controller's
                    // `ibc_packet_ack`, stranding the relayer and freezing the
                    // lease in `OpenLease`. Treat it as an open failure
                    // instead: refund the customer and move to the terminal
                    // `OpenFailed` so the ack commits and operators see a
                    // `wasm-ls-remote-lease-open-failed` event to audit.
                    self.on_open_failed(
                        querier,
                        &env,
                        RemoteErrorMessage::from_static(UNEXPECTED_OPERATION_REASON),
                    )
                }
                RemoteLeaseCallback::OperationErr(reason) => {
                    self.on_open_failed(querier, &env, reason)
                }
                RemoteLeaseCallback::OperationTimeout => self.on_open_failed(
                    querier,
                    &env,
                    RemoteErrorMessage::from_static(TIMEOUT_REASON),
                ),
            })
    }
}

struct RepayOpenLoan {
    principal: LpnCoin,
    now: Instant,
    reserve: ReserveRef,
}

impl WithLppLoan<LpnCurrency> for RepayOpenLoan {
    type Output = Batch;
    type Error = ContractError;

    fn exec<Loan>(self, mut loan: Loan) -> Result<Self::Output, Self::Error>
    where
        Loan: LppLoan<LpnCurrency>,
    {
        let interest = loan.interest_due(&self.now).ok_or(ContractError::Overflow(
            "Open-loan refund due interest overflow",
        ))?;
        let total_repay = self
            .principal
            .checked_add(interest)
            .ok_or(ContractError::Overflow(
                "Open-loan refund due total repay overflow",
            ))?;

        let reserve_cover_batch = reserve_batch(self.reserve, interest)?;

        let receipt = loan.repay(&self.now, total_repay);
        debug_assert_eq!(
            Some(RepayShares {
                principal: self.principal,
                interest,
                excess: Coin::new(Zero::ZERO),
            }),
            receipt
        );

        loan.try_into()
            .map(|batch: lpp::stub::LppBatch<lpp::stub::LppRef<LpnCurrency>>| batch.batch)
            .map_err(ContractError::from)
            // the reserve cover must precede the repay so the interest lands
            // before Lpp pulls the combined principal and interest
            .map(|lpp_batch| reserve_cover_batch.merge(lpp_batch))
    }
}

fn reserve_batch(reserve: ReserveRef, interest: Coin<LpnCurrency>) -> Result<Batch, ContractError> {
    if !interest.is_zero() {
        let mut reserve = reserve.into_reserve();
        reserve.cover_liquidation_losses(interest);
        reserve.try_into().map_err(ContractError::from)
    } else {
        Ok(Batch::default())
    }
}

struct SendToCustomer {
    customer: Addr,
}

impl WithCoin<PaymentGroup> for SendToCustomer {
    type Outcome = Batch;

    fn on<C>(self, amount: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<PaymentGroup> + MemberOf<<PaymentGroup as Group>::TopG>,
    {
        let mut sender = LazySenderStub::new(self.customer);
        sender.send(amount);
        sender.into()
    }
}
