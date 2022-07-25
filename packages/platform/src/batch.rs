use cosmwasm_std::{to_binary, Addr, CosmosMsg, Response, SubMsg, WasmMsg};
use finance::{coin::Coin, currency::Currency};
use serde::Serialize;

use crate::{coin_legacy::to_cosmwasm_impl, error::Result};

#[derive(Default)]
pub struct Batch {
    msgs: Vec<SubMsg>,
}

impl Batch {
    pub fn schedule_execute_no_reply<M>(&mut self, msg: M)
    where
        M: Into<CosmosMsg>,
    {
        let msg_cw = SubMsg::new(msg);

        self.msgs.push(msg_cw);
    }

    pub fn schedule_execute_wasm_no_reply<M, C>(
        &mut self,
        addr: &Addr,
        msg: M,
        funds: Option<Coin<C>>,
    ) -> Result<()>
    where
        M: Serialize,
        C: Currency,
    {
        let wasm_msg = Self::wasm_exec_msg(addr, msg, funds)?;
        let msg_cw = SubMsg::new(wasm_msg);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn schedule_execute_wasm_on_success_reply<M, C>(
        &mut self,
        addr: &Addr,
        msg: M,
        funds: Option<Coin<C>>,
        reply_id: u64,
    ) -> Result<()>
    where
        M: Serialize,
        C: Currency,
    {
        let wasm_msg = Self::wasm_exec_msg(addr, msg, funds)?;
        let msg_cw = SubMsg::reply_on_success(wasm_msg, reply_id);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn merge(self, mut other: Batch) -> Self {
        let mut res = self;
        res.msgs.append(&mut other.msgs);
        res
    }

    fn wasm_exec_msg<M, C>(addr: &Addr, msg: M, funds: Option<Coin<C>>) -> Result<WasmMsg>
    where
        M: Serialize,
        C: Currency,
    {
        let msg_bin = to_binary(&msg)?;
        let mut funds_cw = vec![];
        if let Some(coin) = funds {
            funds_cw.push(to_cosmwasm_impl(coin));
        }

        Ok(WasmMsg::Execute {
            contract_addr: addr.into(),
            funds: funds_cw,
            msg: msg_bin,
        })
    }
}

impl From<Batch> for Response {
    fn from(p: Batch) -> Self {
        let res = Self::default();
        p.msgs
            .into_iter()
            .fold(res, |res, msg| res.add_submessage(msg))
    }
}
