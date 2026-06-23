use currencies::PaymentGroup;
use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, WithCoin},
    instant::Instant,
};
use lpp::stub::loan::{LppLoan, WithLppLoan};
use platform::{
    bank::{FixedAddressSender, LazySenderStub},
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::callback::RemoteErrorMessage;
use reserve::stub::Reserve as ReserveTrait;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::DownpaymentCoin,
    contract::{
        finalize::LeasesRef,
        state::{Response, State, open_failed::OpenFailed},
    },
    error::{ContractError, ContractResult},
    finance::{LpnCoin, LpnCurrency, LppRef, ReserveRef},
};

const OPEN_FAILED_EVENT: &str = "ls-remote-lease-open-failed";

const INTEREST_OVERFLOW: &str = "the failed-open loan interest overflowed";

/// The funds and references a failed open refunds against
pub(super) struct OpenFailureRefund {
    pub downpayment: DownpaymentCoin,
    pub principal: LpnCoin,
    pub customer: Addr,
    pub reserve: ReserveRef,
    pub lpp_ref: LppRef,
    pub leases_ref: LeasesRef,
    pub lease_addr: Addr,
    pub now: Instant,
}

/// Refund a failed open and move to the `OpenFailed` terminal
///
/// Closes the LPP loan in full — principal plus the interest that accrued
/// since it was drawn — refunds the whole downpayment to the customer, and
/// finalises the lease. The interest is covered from the reserve, mirroring
/// the live-lease full-close path: the reserve message is scheduled first so
/// its LPN lands on the lease account before the LPP repay pulls it, and the
/// whole sequence is in-transaction with no reply handler. If the reserve
/// cannot cover the interest the covering message reverts the transaction; the
/// relayer retries once the reserve is funded — the same blocks-until-funded
/// posture a live liquidation has.
///
/// Used by both failed-open paths: the synchronous one (the OpenLease packet
/// itself failed, funds never left) and the unwind one (the opening swap
/// failed and the inputs were drained back home). The same residual-interest
/// close applies to both.
pub(super) fn refund_to_open_failed(
    refund: OpenFailureRefund,
    reason: RemoteErrorMessage,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let OpenFailureRefund {
        downpayment,
        principal,
        customer,
        reserve,
        lpp_ref,
        leases_ref,
        lease_addr,
        now,
    } = refund;
    lpp_ref
        .execute_loan(
            RepayOpenLoan { principal, now },
            lease_addr.clone(),
            querier,
        )
        .and_then(
            |FullLoanRepayment {
                 batch: lpp_batch,
                 interest,
             }| {
                cover_interest(reserve, interest).map(|reserve_batch| (lpp_batch, reserve_batch))
            },
        )
        .and_then(|(lpp_batch, reserve_batch)| {
            let customer_batch = downpayment.with_coin(SendToCustomer {
                customer: customer.clone(),
            });
            leases_ref.finalize_lease(customer).map(|finalize_batch| {
                // The reserve covers the interest before the LPP pulls it, so its
                // message must precede the repay; the refund and finalize follow.
                reserve_batch
                    .merge(lpp_batch)
                    .merge(customer_batch)
                    .merge(finalize_batch)
            })
        })
        .map(|batch| open_failed_response(batch, lease_addr, reason, leases_ref))
}

fn cover_interest(reserve: ReserveRef, interest: LpnCoin) -> ContractResult<Batch> {
    // A failed open whose loan was drawn in the same block accrues no interest;
    // a zero-amount cover would schedule a zero-coin reserve send that the bank
    // module rejects, reverting the whole refund. Skip the cover instead — the
    // repay closes the loan on the principal alone.
    if interest.is_zero() {
        Ok(Batch::default())
    } else {
        let mut reserve = reserve.into_reserve();
        reserve.cover_liquidation_losses(interest);
        reserve.try_into().map_err(Into::into)
    }
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

/// The LPP repay batch closing a failed-open loan and the interest it covered
///
/// `interest` is the residual due interest the reserve must cover so the repay
/// closes the loan; the caller schedules the reserve cover for it ahead of the
/// repay batch.
struct FullLoanRepayment {
    batch: Batch,
    interest: LpnCoin,
}

/// Repay a failed-open LPP loan in full — principal plus accrued interest
///
/// The interest is read from the live loan at `now` rather than assumed zero:
/// on the unwind path the loan accrues over the drain window, and the LPP
/// `repay` is interest-first, so repaying only the principal would leave the
/// interest, and an equal amount of principal, outstanding. Repaying
/// `principal + interest_due` closes the loan; the interest portion is covered
/// from the reserve by the caller.
struct RepayOpenLoan {
    principal: LpnCoin,
    now: Instant,
}

impl WithLppLoan<LpnCurrency> for RepayOpenLoan {
    type Output = FullLoanRepayment;
    type Error = ContractError;

    fn exec<Loan>(self, mut loan: Loan) -> Result<Self::Output, Self::Error>
    where
        Loan: LppLoan<LpnCurrency>,
    {
        loan.interest_due(&self.now)
            .ok_or(ContractError::Overflow(INTEREST_OVERFLOW))
            .and_then(|interest| {
                let _receipt = loan.repay(&self.now, self.principal + interest);
                loan.try_into()
                    .map(
                        |batch: lpp::stub::LppBatch<lpp::stub::LppRef<LpnCurrency>>| {
                            FullLoanRepayment {
                                batch: batch.batch,
                                interest,
                            }
                        },
                    )
                    .map_err(Into::into)
            })
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
