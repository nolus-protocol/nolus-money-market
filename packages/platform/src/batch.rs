use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use sdk::{
    cosmwasm_ext::{CosmosMsg, Response, SubMsg},
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
        admin: Option<String>,
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

    pub fn into_emitter<T>(self, event_type: T) -> Emitter
    where
        T: Into<String>,
    {
        Emitter::new(self, event_type)
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
        admin: Option<String>,
    ) -> Result<WasmMsg>
    where
        M: Serialize,
    {
        let msg_bin = to_binary(&msg)?;
        let mut funds_cw = vec![];
        if let Some(coin) = funds {
            funds_cw = coin;
        }

        Ok(WasmMsg::Instantiate {
            admin,
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

impl From<Batch> for Response {
    fn from(p: Batch) -> Self {
        p.msgs
            .into_iter()
            .fold(Self::default(), |res, msg| res.add_submessage(msg))
    }
}

#[cfg(test)]
mod test {
    use sdk::{
        cosmwasm_ext::{CosmosMsg, Response},
        cosmwasm_std::{Event, WasmMsg},
    };

    use crate::emit::Emit;

    use super::Batch;

    const TY1: &str = "E_TYPE";
    const KEY1: &str = "my_event_key";
    const KEY2: &str = "my_other_event_key";
    const VALUE1: &str = "my_event_value";
    const VALUE2: &str = "my_other_event_value";

    #[test]
    fn no_events() {
        let mut b = Batch::default();
        b.schedule_execute_no_reply(CosmosMsg::Wasm(WasmMsg::ClearAdmin {
            contract_addr: "".to_string(),
        }));
        let resp: Response = b.into();
        assert_eq!(1, resp.messages.len());
        assert_eq!(0, resp.attributes.len());
        assert_eq!(0, resp.events.len());
    }

    #[test]
    fn emit() {
        let e = Batch::default().into_emitter(TY1).emit(KEY1, VALUE1);
        let resp: Response = e.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1).add_attribute(KEY1, VALUE1);
        assert_eq!(exp, resp.events[0]);
    }

    #[test]
    fn emit_same_attr() {
        let e = Batch::default()
            .into_emitter(TY1)
            .emit(KEY1, VALUE1)
            .emit(KEY1, VALUE1);
        let resp: Response = e.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1)
            .add_attribute(KEY1, VALUE1)
            .add_attribute(KEY1, VALUE1);
        assert_eq!(exp, resp.events[0]);
    }

    #[test]
    fn emit_two_attrs() {
        let e = Batch::default()
            .into_emitter(TY1)
            .emit(KEY1, VALUE1)
            .emit(KEY2, VALUE2);
        let resp: Response = e.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1)
            .add_attribute(KEY1, VALUE1)
            .add_attribute(KEY2, VALUE2);
        assert_eq!(exp, resp.events[0]);
    }
}
