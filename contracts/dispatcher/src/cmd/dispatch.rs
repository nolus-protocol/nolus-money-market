use crate::state::Config;
use crate::ContractError;

use cosmwasm_std::StdResult;
use cosmwasm_std::Timestamp;
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::price::total_of;

use lpp::stub::{Lpp as LppTrait, WithLpp};
use marketprice::storage::Price;
use platform::batch::{Emit, Emitter};
use serde::Serialize;

use super::dispatcher::Dispatcher;
use super::Dispatch;

impl WithLpp for Dispatch {
    type Output = Emitter;
    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        let amount_native: Coin<Nls> = self.price.quote().amount.into();
        let amount: Coin<Lpn> = self.price.base().amount.into();

        let native_price = total_of(amount_native).is(amount);

        let result = Dispatcher::new(lpp, self.last_dispatch, self.config, self.block_time)?
            .dispatch(native_price)?;
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

impl Dispatch {
    pub fn new(
        last_dispatch: Timestamp,
        price: Price,
        config: Config,
        block_time: Timestamp,
    ) -> StdResult<Dispatch> {
        Ok(Self {
            last_dispatch,
            price,
            config,
            block_time,
        })
    }
}
