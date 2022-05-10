use cosmwasm_std::{to_binary, Addr, Api, Coin, StdResult, SubMsg, WasmMsg};
use serde::{Serialize, Deserialize};

use crate::msg::ExecuteMsg;

pub const REPLY_ID: u64 = 28;

pub trait Lpp {
    fn open_loan_async(&mut self, amount: Coin) -> StdResult<()>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LppStub {
    addr: Addr,
    open_loan_msg: Option<SubMsg>,
}

impl LppStub {
    pub fn try_from<A>(addr_raw: String, api: &A) -> StdResult<Self>
    where
        A: ?Sized + Api,
    {
        let addr = api.addr_validate(&addr_raw)?;
        Ok(Self {
            addr,
            open_loan_msg: None,
        })
    }
}

impl Lpp for LppStub {
    fn open_loan_async(&mut self, amount: Coin) -> StdResult<()> {
        let msg = to_binary(&ExecuteMsg::OpenLoan { amount })?;
        let cosmos_sub_msg = SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: self.addr.as_ref().into(),
                funds: vec![],
                msg,
            },
            REPLY_ID,
        );
        let old_msg = self.open_loan_msg.replace(cosmos_sub_msg);
        debug_assert!(old_msg.is_none(), "Double opening a loan!");
        Ok(())
    }
}
