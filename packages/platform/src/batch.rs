use cosmwasm_std::{to_binary, Addr, CosmosMsg, Event, Response, SubMsg, Timestamp, WasmMsg};
use finance::{coin::Coin, currency::Currency};
use serde::Serialize;

use crate::{coin_legacy::to_cosmwasm_impl, error::Result};

#[derive(Default)]
pub struct Batch {
    msgs: Vec<SubMsg>,
    event: Option<Event>,
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

    pub fn emit<T, K, V>(&mut self, event_type: T, event_key: K, event_value: V)
    where
        T: Into<String>,
        K: Into<String>,
        V: Into<String>,
    {
        // do not use Option.get_or_insert_with(f) since asserting on the type would require clone of the type
        if self.event.is_none() {
            self.event = Some(Event::new(event_type));
        } else {
            debug_assert!(
                self.event.as_ref().unwrap().ty == event_type.into(),
                "The platform batch supports only one event type"
            );
        }
        let event = self.event.take().expect("empty event");
        let none = self
            .event
            .replace(event.add_attribute(event_key, event_value));
        debug_assert!(none.is_none());
    }


    /// Specialization of [`emit`](Batch::emit) for timestamps.
    pub fn emit_timestamp<T, K>(&mut self, event_type: T, event_key: K, timestamp: &Timestamp)
    where
        T: Into<String>,
        K: Into<String>,
    {
        self.emit(event_type, event_key, timestamp.nanos().to_string())
    }


    pub fn emit_coin<T,K,C>(&mut self, event_type: T, event_key: K, coin: Coin<C>)
    where
        T: Into<String> + Clone,
        K: Into<String>,
        C: Currency,
    {
        let key = event_key.into();
        let amount_key = key.clone() + "-amount";
        let symbol_key = key + "-symbol";
        
        self.emit(event_type.clone(), amount_key, u128::from(coin).to_string());
        self.emit(event_type, symbol_key, C::SYMBOL)
    }

    pub fn emit_amount<T,K,C>(&mut self, event_type: T, event_key: K, coin: Coin<C>)
    where
        T: Into<String>,
        K: Into<String>,
        C: Currency,
    {  
        self.emit(event_type, event_key, u128::from(coin).to_string())
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
        let res = p
            .msgs
            .into_iter()
            .fold(Self::default(), |res, msg| res.add_submessage(msg));
        p.event.into_iter().fold(res, |res, e| res.add_event(e))
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{CosmosMsg, Empty, Event, Response};

    use super::Batch;

    const TY1: &str = "E_TYPE";
    const KEY1: &str = "my_event_key";
    const KEY2: &str = "my_other_event_key";
    const VALUE1: &str = "my_event_value";
    const VALUE2: &str = "my_other_event_value";

    #[test]
    fn no_events() {
        let mut b = Batch::default();
        b.schedule_execute_no_reply(CosmosMsg::Custom(Empty {}));
        let resp: Response = b.into();
        assert_eq!(1, resp.messages.len());
        assert_eq!(0, resp.attributes.len());
        assert_eq!(0, resp.events.len());
    }

    #[test]
    fn emit() {
        let mut b = Batch::default();
        b.emit(TY1, KEY1, VALUE1);
        let resp: Response = b.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1).add_attribute(KEY1, VALUE1);
        assert_eq!(exp, resp.events[0]);
    }

    #[test]
    fn emit_same_attr() {
        let mut b = Batch::default();
        b.emit(TY1, KEY1, VALUE1);
        b.emit(TY1, KEY1, VALUE1);
        let resp: Response = b.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1)
            .add_attribute(KEY1, VALUE1)
            .add_attribute(KEY1, VALUE1);
        assert_eq!(exp, resp.events[0]);
    }

    #[test]
    fn emit_two_attrs() {
        let mut b = Batch::default();
        b.emit(TY1, KEY1, VALUE1);
        b.emit(TY1, KEY2, VALUE2);
        let resp: Response = b.into();
        assert_eq!(1, resp.events.len());
        let exp = Event::new(TY1)
            .add_attribute(KEY1, VALUE1)
            .add_attribute(KEY2, VALUE2);
        assert_eq!(exp, resp.events[0]);
    }
}
