use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};

use crate::msg::ExecuteMsg;

pub struct LppStub {
    addr: Addr,
}

impl LppStub {
    pub fn from<T: Into<Addr>>(addr: T) -> Self {
        Self { addr: addr.into() }
    }

    pub fn create_open_loan_msg(&self, amount: Coin) -> StdResult<CosmosMsg> {
        let msg = to_binary(&ExecuteMsg::OpenLoan { amount })?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr.as_ref().into(),
            funds: vec![],
            msg,
        }
        .into())
    }
}
