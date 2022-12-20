use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::{currency::Currency, percent::Percent, price::Price};
use sdk::{
    cosmwasm_std::{StdResult, Storage, Uint64},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{
    error::{ContractError, ContractResult},
    nlpn::NLpn,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub lpn_ticker: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Percent,
    pub utilization_optimal: Percent,
    pub addon_optimal_interest_rate: Percent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(
        lpn_ticker: String,
        lease_code_id: Uint64,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> ContractResult<Self> {
        Self::validate_input(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )?;

        Ok(Config {
            lpn_ticker,
            lease_code_id,
            base_interest_rate: Percent::from_percent(7),
            utilization_optimal: Percent::from_percent(70),
            addon_optimal_interest_rate: Percent::from_percent(2),
        })
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        &mut self,
        storage: &mut dyn Storage,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> ContractResult<()> {
        Self::validate_input(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )?;

        self.base_interest_rate = base_interest_rate;
        self.utilization_optimal = utilization_optimal;
        self.addon_optimal_interest_rate = addon_optimal_interest_rate;

        self.store(storage).map_err(Into::into)
    }

    pub fn initial_derivative_price<LPN>() -> Price<NLpn, LPN>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
    }

    fn validate_input(
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> ContractResult<()> {
        if base_interest_rate > Percent::HUNDRED {
            return Err(ContractError::InvalidConfigParameter(
                "Base interest rate should not be greater than 100%!",
            ));
        }

        if utilization_optimal > Percent::HUNDRED {
            return Err(ContractError::InvalidConfigParameter(
                "Optimal utilization should not be greater than 100%!",
            ));
        }

        if addon_optimal_interest_rate > Percent::HUNDRED {
            return Err(ContractError::InvalidConfigParameter(
                "Addon optimal interest rate should not be greater than 100%!",
            ));
        }

        Ok(())
    }
}
