use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{ContractError, ContractResult};

pub trait Finalizer
where
    Self: TryInto<Batch>,
{
    fn on_finish(&mut self);
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    FinalizeLease {},
}

#[derive(Serialize, Deserialize)]
pub struct FinalizerRef {
    addr: Addr,
}

impl FinalizerRef {
    #[allow(unused)]
    pub(super) fn try_new(addr: Addr, querier: &QuerierWrapper<'_>) -> ContractResult<Self> {
        use platform::contract;

        contract::validate_addr(querier, &addr)
            .map(|()| Self { addr })
            .map_err(Into::into)
    }

    #[allow(unused)]
    pub(super) fn into_stub(self) -> impl Finalizer {
        Stub::from(self)
    }
}

impl From<FinalizerRef> for Stub {
    fn from(reff: FinalizerRef) -> Self {
        Self {
            reff,
            do_call: false,
        }
    }
}

struct Stub {
    reff: FinalizerRef,
    do_call: bool,
}

impl Finalizer for Stub {
    fn on_finish(&mut self) {
        self.do_call = true;
    }
}

impl TryFrom<Stub> for Batch {
    type Error = ContractError;

    fn try_from(stub: Stub) -> ContractResult<Self> {
        let mut msgs = Batch::default();
        if stub.do_call {
            msgs.schedule_execute_wasm_no_reply_no_funds(
                &stub.reff.addr,
                ExecuteMsg::FinalizeLease {},
            )?;
        }
        Ok(msgs)
    }
}
