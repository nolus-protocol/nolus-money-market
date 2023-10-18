use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{error::Result, msg::QueryMsg, Lpp, LppBalanceResponse};

pub struct LppStub<'a> {
    lpp: Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> LppStub<'a> {
    pub(crate) fn new(lpp: Addr, querier: &'a QuerierWrapper<'a>) -> Self {
        Self { lpp, querier }
    }
}

impl<'a> Lpp for LppStub<'a> {
    fn balance(&self) -> Result<LppBalanceResponse> {
        let msg = QueryMsg::LppBalance {};
        self.querier
            .query_wasm_smart(self.lpp.clone(), &msg)
            .map_err(Into::into)
    }
}
