use cosmwasm_std::StdResult;
use serde::Serialize;

use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::price::{Price, total};
use oracle::stub::{Oracle as OracleTrait, WithOracle};

use crate::ContractError;

use super::PriceConvert;

impl<Lpn> WithOracle<Lpn> for PriceConvert<Lpn>
where
    Lpn: Currency + Serialize,
{
    type Output = Coin<Nls>;
    type Error = ContractError;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let price_dto = oracle.get_price(Nls::SYMBOL.to_string())?.price;
        let converted = Price::<Nls, Lpn>::try_from(price_dto)?.inv();
        let reward_unls: Coin<Nls> = total(self.amount, converted);
        Ok(reward_unls)
    }

    fn unexpected_base(
        self,
        symbol: finance::currency::SymbolOwned,
    ) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

impl<Lpn> PriceConvert<Lpn>
where
    Lpn: Currency,
{
    pub fn with(amount: Coin<Lpn>) -> StdResult<PriceConvert<Lpn>> {
        Ok(Self { amount })
    }
}
