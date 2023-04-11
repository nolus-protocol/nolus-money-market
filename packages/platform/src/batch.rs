use std::vec;

use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use sdk::{
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{to_binary, Addr, Coin as CoinCw, WasmMsg},
};

pub use crate::emit::{Emit, Emitter};
use crate::{coin_legacy::to_cosmwasm_impl, error::Result};

pub type ReplyId = u64;

#[derive(Default)]
#[cfg_attr(
    any(debug_assertions, test, feature = "testing"),
    derive(Debug, PartialEq)
)]
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

    pub fn schedule_execute_on_success_reply<M>(&mut self, msg: M, reply_id: ReplyId)
    where
        M: Into<CosmosMsg>,
    {
        let msg_cw = SubMsg::reply_on_success(msg, reply_id);

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
        reply_id: ReplyId,
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

    pub fn schedule_execute_wasm_reply_always<M, C>(
        &mut self,
        addr: &Addr,
        msg: M,
        funds: Option<Coin<C>>,
        reply_id: ReplyId,
    ) -> Result<()>
    where
        M: Serialize,
        C: Currency,
    {
        let wasm_msg = Self::wasm_exec_msg(addr, msg, funds)?;
        let msg_cw = SubMsg::reply_always(wasm_msg, reply_id);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn schedule_execute_wasm_reply_error<M, C>(
        &mut self,
        addr: &Addr,
        msg: M,
        funds: Option<Coin<C>>,
        reply_id: ReplyId,
    ) -> Result<()>
    where
        M: Serialize,
        C: Currency,
    {
        let wasm_msg = Self::wasm_exec_msg(addr, msg, funds)?;
        let msg_cw = SubMsg::reply_on_error(wasm_msg, reply_id);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn schedule_instantiate_wasm_on_success_reply<M>(
        &mut self,
        code_id: u64,
        msg: M,
        funds: Option<Vec<CoinCw>>,
        label: &str,
        admin: Option<Addr>,
        reply_id: ReplyId,
    ) -> Result<()>
    where
        M: Serialize,
    {
        let wasm_msg = Self::wasm_init_msg(code_id, msg, funds, label, admin)?;
        let msg_cw = SubMsg::reply_on_success(wasm_msg, reply_id);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn schedule_migrate_wasm_no_reply<M>(
        &mut self,
        addr: &Addr,
        msg: M,
        new_code_id: u64,
    ) -> Result<()>
    where
        M: Serialize,
    {
        let wasm_msg = Self::wasm_migrate_msg(addr, msg, new_code_id)?;
        let msg_cw = SubMsg::new(wasm_msg);

        self.msgs.push(msg_cw);
        Ok(())
    }

    pub fn merge(self, mut other: Batch) -> Self {
        let mut res = self;
        res.msgs.append(&mut other.msgs);
        res
    }

    pub fn len(&self) -> usize {
        self.msgs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.msgs.is_empty()
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

    fn wasm_init_msg<M>(
        code_id: u64,
        msg: M,
        funds: Option<Vec<CoinCw>>,
        label: &str,
        admin: Option<Addr>,
    ) -> Result<WasmMsg>
    where
        M: Serialize,
    {
        let admin_str = admin.map(Into::into);
        let msg_bin = to_binary(&msg)?;
        let mut funds_cw = vec![];
        if let Some(coin) = funds {
            funds_cw = coin;
        }

        Ok(WasmMsg::Instantiate {
            admin: admin_str,
            code_id,
            funds: funds_cw,
            label: label.to_string(),
            msg: msg_bin,
        })
    }

    fn wasm_migrate_msg<M>(addr: &Addr, msg: M, new_code_id: u64) -> Result<WasmMsg>
    where
        M: Serialize,
    {
        let msg_bin = to_binary(&msg)?;

        Ok(WasmMsg::Migrate {
            contract_addr: addr.into(),
            new_code_id,
            msg: msg_bin,
        })
    }
}

impl IntoIterator for Batch {
    type Item = SubMsg;

    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.msgs.into_iter()
    }
}

#[cfg(test)]
mod test {
    use sdk::{cosmwasm_ext::CosmosMsg, cosmwasm_std::WasmMsg};

    use super::Batch;

    #[test]
    fn no_events() {
        let mut b = Batch::default();
        assert_eq!(0, b.len());
        assert!(b.is_empty());

        b.schedule_execute_no_reply(CosmosMsg::Wasm(WasmMsg::ClearAdmin {
            contract_addr: "".to_string(),
        }));
        assert_eq!(1, b.len());
        assert!(!b.is_empty());
    }

    #[test]
    fn msgs_len() {
        let mut b = Batch::default();
        assert_eq!(0, b.len());
        assert!(b.is_empty());
        b.schedule_execute_no_reply(CosmosMsg::Wasm(WasmMsg::ClearAdmin {
            contract_addr: "".into(),
        }));
        assert_eq!(1, b.len());
        b.schedule_execute_no_reply(CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
            contract_addr: "".into(),
            admin: "".into(),
        }));
        assert_eq!(2, b.len());
        assert!(!b.is_empty());
    }
}
