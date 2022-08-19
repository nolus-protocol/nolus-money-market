use crate::ContractError;

use cosmwasm_std::StdResult;
use finance::currency::{Currency, Nls};

use oracle::msg::PriceResponse;
use oracle::stub::{Oracle as OracleTrait, WithOracle};

use serde::Serialize;

use super::GetPrice;

impl WithOracle for GetPrice {
    type Output = PriceResponse;
    type Error = ContractError;

    fn exec<Lpn: 'static, Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        Ok(oracle.get_price(vec![Nls::SYMBOL.to_string()])?)
    }

    fn unknown_lpn(
        self,
        symbol: finance::currency::SymbolOwned,
    ) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

impl GetPrice {
    pub fn new() -> StdResult<GetPrice> {
        Ok(Self {})
    }
}
