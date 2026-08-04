use crate::api::LeasePaymentCurrencies;
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{ContractError, ContractResult};

use super::{Response, State};

/// Sweep any funds stranded on a terminal lease to whoever calls `Heal()`.
///
/// Deliberately unauthenticated: a terminal lease should hold nothing, so a
/// non-empty balance is an inconsistency with no owner to return it to, and
/// the caller-takes-all sweep is the incentive for someone to clean it up.
/// The lease state does not change; with nothing to sweep the call fails
/// with `InconsistencyNotDetected`.
pub(super) trait DrainAll
where
    Self: Into<State>,
{
    fn drain(self, from: &Addr, to: Addr, querier: QuerierWrapper<'_>) -> ContractResult<Response> {
        bank::bank_send_all::<LeasePaymentCurrencies>(from, to, querier)
            .map_err(Into::into)
            .and_then(|msgs| {
                if msgs.is_empty() {
                    Err(ContractError::InconsistencyNotDetected())
                } else {
                    Ok(msgs)
                }
            })
            .map(|msgs| Response::from(msgs, self))
    }
}
