use serde::{Deserialize, Serialize, Serializer};

use finance::{liability::Liability, percent::Percent};
use lease::api::{ConnectionParams, InterestPaymentSpec, LpnCoin, PositionSpecDTO};
use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};

use crate::result::ContractResult;

use super::config::Config as ConfigNew;

#[derive(Deserialize)]
pub struct Config {
    pub lease_code_id: CodeId,
    pub lpp_addr: Addr,
    pub lease_interest_rate_margin: Percent,
    pub liability: Liability,
    pub lease_interest_payment: InterestPaymentSpec,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub profit: Addr,
    pub dex: Option<ConnectionParams>,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn migrate(
        storage: &dyn Storage,
        min_asset: LpnCoin,
        min_sell_asset: LpnCoin,
    ) -> ContractResult<ConfigNew> {
        Self::STORAGE
            .load(storage)
            .map_err(Into::into)
            .map(|config| ConfigNew {
                lease_code_id: config.lease_code_id,
                lpp_addr: config.lpp_addr,
                lease_interest_rate_margin: config.lease_interest_rate_margin,
                lease_position_spec: PositionSpecDTO::new(
                    config.liability,
                    min_asset,
                    min_sell_asset,
                ),
                lease_interest_payment: config.lease_interest_payment,
                time_alarms: config.time_alarms,
                market_price_oracle: config.market_price_oracle,
                profit: config.profit,
                dex: config.dex,
            })
    }
}

impl Serialize for Config {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
