use serde::{Deserialize, Serialize, Serializer};

use finance::percent::Percent;
use lease::api::open::{ConnectionParams, InterestPaymentSpec, PositionSpecDTO};
use platform::contract::CodeId;
use sdk::{cosmwasm_ext::as_dyn::storage, cosmwasm_std::Addr, cw_storage_plus::Item};

use crate::result::ContractResult;

use super::config::Config as ConfigNew;

#[derive(Deserialize)]
pub struct Config {
    pub lease_code_id: CodeId,
    pub lpp_addr: Addr,
    pub lease_interest_rate_margin: Percent,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_interest_payment: InterestPaymentSpec,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub profit: Addr,
    pub dex: Option<ConnectionParams>,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn migrate<S>(storage: &S) -> ContractResult<ConfigNew>
    where
        S: storage::Dyn + ?Sized,
    {
        Self::STORAGE
            .load(storage.as_dyn())
            .map_err(Into::into)
            .map(|config| ConfigNew {
                lease_code_id: config.lease_code_id,
                lpp: config.lpp_addr,
                profit: config.profit,
                time_alarms: config.time_alarms,
                market_price_oracle: config.market_price_oracle,
                lease_position_spec: config.lease_position_spec,
                lease_interest_rate_margin: config.lease_interest_rate_margin,
                lease_due_period: config.lease_interest_payment.due_period(),
                dex: config.dex.expect("dex connection should have been set"),
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
