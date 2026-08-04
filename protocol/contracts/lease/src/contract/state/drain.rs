use crate::api::LeasePaymentCurrencies;
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{ContractError, ContractResult};

use super::{Response, State};

// Deliberately unauthenticated caller-takes-all sweep: a terminal lease
// should hold nothing, an inconsistent balance has no owner to return it
// to, and rewarding whoever calls `Heal()` is the incentive to clean it up.
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
