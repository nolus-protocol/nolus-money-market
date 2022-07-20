use crate::dispatcher::Dispatcher;
use crate::state::config::Config;
use crate::ContractError;
use cosmwasm_std::{QuerierWrapper, Response, Timestamp};
use cosmwasm_std::{StdResult, Storage};
use finance::currency::Currency;

use lpp::stub::{Lpp as LppTrait, WithLpp};
use serde::Serialize;

pub struct DispatcherRef<'a> {
    storage: &'a mut dyn Storage,
    querier: QuerierWrapper<'a>,
    config: Config,
    block_time: Timestamp,
}

impl<'a> WithLpp for DispatcherRef<'a> {
    type Output = Response;
    type Error = ContractError;

    fn exec<C, L>(self, lpp: L) -> Result<Self::Output, Self::Error>
    where
        L: LppTrait<C>,
        C: Currency + Serialize,
    {
        Dispatcher::new(
            lpp,
            self.storage,
            self.querier,
            self.config,
            self.block_time,
        )?
        .dispatch()
    }

    fn unknown_lpn(
        self,
        symbol: finance::currency::SymbolOwned,
    ) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

impl<'a> DispatcherRef<'a> {
    pub fn new(
        storage: &'a mut dyn Storage,
        querier: QuerierWrapper<'a>,
        config: Config,
        block_time: Timestamp,
    ) -> StdResult<DispatcherRef<'a>> {
        Ok(Self {
            storage,
            querier,
            config,
            block_time,
        })
    }
}
