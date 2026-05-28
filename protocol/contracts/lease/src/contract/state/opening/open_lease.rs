use currencies::PaymentGroup;
use currency::{CurrencyDef, Group, MemberOf};
use cw_time::IntoInstant;
use finance::{
    coin::{Coin, WithCoin},
    duration::Duration,
    instant::Instant,
};
use lpp::stub::loan::{LppLoan, WithLppLoan};
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
    stub::{ControllerInnerMessage, Factory},
};
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};
use serde::{Deserialize, Serialize};
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
        state::{Response, State, closed::Closed},
    },
    error::{ContractError, ContractResult},
    finance::{LpnCoin, LpnCurrency, LppRef, OracleRef},
};

const OPEN_FAILED_EVENT: &str = "ls-remote-lease-open-failed";

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
                .open(params, OpenLeaseParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::OpenLease { params, timeout }
                })
                .map_err(ContractError::from)
        })
    }

    fn authz_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> ContractResult<()> {
        access_control::check(
            &self.deps.3.remote_lease_callback_permission(querier),
            info,
        )
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
            Some(remote_lease_id),
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
        reason_label: String,
    ) -> ContractResult<Response> {
        let Self {
            new_lease,
            downpayment,
            loan,
            deps: (lpp_ref, _oracle, _time_alarms, leases_ref),
            start_opening_at: _,
        } = self;
        let customer = new_lease.form.customer;
        let lease_addr = env.contract.address.clone();
        let now = env.block.time.into_instant();

        Coin::<LpnCurrency>::try_from(loan.principal)
            .map_err(ContractError::from)
            .and_then(|principal| {
                lpp_ref.execute_loan(RepayOpenLoan { principal, now }, lease_addr.clone(), querier)
            })
            .and_then(|lpp_batch| {
                let customer_batch = downpayment.with_coin(SendToCustomer {
                    customer: customer.clone(),
                });
                leases_ref
                    .finalize_lease(customer)
                    .map(|finalize_batch| lpp_batch.merge(customer_batch).merge(finalize_batch))
            })
            .map(|batch| {
                let emitter = Emitter::of_type(OPEN_FAILED_EVENT)
                    .emit("id", lease_addr)
                    .emit("reason", reason_label);
                StateMachineResponse::from(
                    MessageResponse::messages_with_event(batch, emitter),
                    State::from(Closed::default()),
                )
            })
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
            in_progress: OngoingTrx::OpenIcaAccount {},
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
                RemoteLeaseCallback::OperationOk(other) => {
                    Err(ContractError::unsupported_operation(format!(
                        "open lease unexpected operation response: {other:?}"
                    )))
                }
                RemoteLeaseCallback::OperationErr(reason) => {
                    self.on_open_failed(querier, &env, reason_to_label(&reason))
                }
                RemoteLeaseCallback::OperationTimeout => {
                    self.on_open_failed(querier, &env, "timeout".to_owned())
                }
            })
    }
}

fn reason_to_label(reason: &RemoteErrorMessage) -> String {
    reason.as_str().to_owned()
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    OpenLease {
        params: OpenLeaseParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

struct RepayOpenLoan {
    principal: LpnCoin,
    now: Instant,
}

impl WithLppLoan<LpnCurrency> for RepayOpenLoan {
    type Output = Batch;
    type Error = ContractError;

    fn exec<Loan>(self, mut loan: Loan) -> Result<Self::Output, Self::Error>
    where
        Loan: LppLoan<LpnCurrency>,
    {
        let _receipt = loan.repay(&self.now, self.principal);
        loan.try_into()
            .map(|batch: lpp::stub::LppBatch<lpp::stub::LppRef<LpnCurrency>>| batch.batch)
            .map_err(Into::into)
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
