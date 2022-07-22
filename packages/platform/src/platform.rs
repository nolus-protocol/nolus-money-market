use cosmwasm_std::{to_binary, Addr, Response, StdResult, SubMsg, WasmMsg};
use finance::{coin::Coin, coin_legacy::to_cosmwasm, currency::Currency};
use serde::Serialize;

#[derive(Default)]
pub struct Platform {
    msgs: Vec<SubMsg>,
}

impl Platform {
    pub fn schedule_execute_no_reply<M, C>(
        &mut self,
        addr: &Addr,
        msg: M,
        funds: Coin<C>,
    ) -> StdResult<()>
    where
        M: Serialize,
        C: Currency,
    {
        let msg_bin = to_binary(&msg)?;
        let msg_cw = SubMsg::new(WasmMsg::Execute {
            contract_addr: addr.into(),
            funds: vec![to_cosmwasm(funds)],
            msg: msg_bin,
        });

        self.msgs.push(msg_cw);
        Ok(())
    }
}

impl From<Platform> for Response {
    fn from(p: Platform) -> Self {
        let res = Self::default();
        p.msgs
            .into_iter()
            .fold(res, |res, msg| res.add_submessage(msg))
    }
}
