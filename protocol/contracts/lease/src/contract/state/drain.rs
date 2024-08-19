use currencies::PaymentGroup;
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{ContractError, ContractResult};

use super::{Response, State};

pub(super) trait DrainAll
where
    Self: Into<State>,
{
    fn drain(self, from: &Addr, to: Addr, querier: QuerierWrapper<'_>) -> ContractResult<Response> {
        bank::bank_send_all::<PaymentGroup>(from, to, querier)
            .map_err(Into::into)
            .and_then(|msgs| {
                if msgs.is_empty() {
                    Err(ContractError::InconsistencyNotDetected())
                } else {
                    Ok(dbg!(msgs))
                }
            })
            .map(|msgs| Response::from(msgs, self))
    }
}
