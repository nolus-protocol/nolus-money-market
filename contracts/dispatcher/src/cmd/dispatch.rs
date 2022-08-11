use crate::state::Config;
use crate::ContractError;
use cosmwasm_std::{QuerierWrapper, Timestamp};
use cosmwasm_std::{StdResult, Storage};
use finance::currency::Currency;

use lpp::stub::{Lpp as LppTrait, WithLpp};
use platform::batch::{Emit, Emitter};
use serde::Serialize;

use super::dispatcher::Dispatcher;
use super::Dispatch;

impl<'a> WithLpp for Dispatch<'a> {
    type Output = Emitter;
    type Error = ContractError;

    fn exec<C, L>(self, lpp: L) -> Result<Self::Output, Self::Error>
    where
        L: LppTrait<C>,
        C: Currency + Serialize,
    {
        let result = Dispatcher::new(
            lpp,
            self.storage,
            self.querier,
            self.config,
            self.block_time,
        )?
        .dispatch()?;
        Ok(result
            .batch
            .into_emitter("tr-rewards")
            .emit_coin("rewards-amount", result.receipt.in_nls)
            .emit_coin("rewards-amount", result.receipt.in_stable))
    }

    fn unknown_lpn(
        self,
        symbol: finance::currency::SymbolOwned,
    ) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

impl<'a> Dispatch<'a> {
    pub fn new(
        storage: &'a mut dyn Storage,
        querier: QuerierWrapper<'a>,
        config: Config,
        block_time: Timestamp,
    ) -> StdResult<Dispatch<'a>> {
        Ok(Self {
            storage,
            querier,
            config,
            block_time,
        })
    }
}
