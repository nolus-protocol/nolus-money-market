use serde::{Deserialize, Serialize};

use currency::SymbolOwned;
use finance::{liability::LiabilityDTO, percent::Percent};
use lease::api::{ConnectionParams, DownpaymentCoin, InterestPaymentSpec, LeaseCoin};
use lpp::msg::LpnCoin;
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    schemars::{self, JsonSchema},
};

use crate::state::config::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub lease_code_id: Uint64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: Percent,
    pub liability: LiabilityDTO,
    pub lease_interest_payment: InterestPaymentSpec,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub profit: Addr,
}

#[derive(Serialize, Deserialize)]
pub struct MigrateMsg {}

pub type MaxLeases = u32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OpenLease {
        currency: SymbolOwned,
        #[serde(default)]
        max_ltd: Option<Percent>,
    },
    /// Start a Lease migration
    ///
    /// The consumed gas is a limitaton factor for the maximum lease instances that
    /// can be processed in a transaction. For that reason, the process does the migration
    /// in batches. A new batch is started with this transaction. It processes the specified
    /// maximum number of leases and emits a continuation key as an event
    /// 'wasm-migrate-leases.contunuation-key=<key>'. That key should be provided
    /// with the next `MigrateLeasesCont` message. It in turn emits
    /// a continuation key with the same event and the procedure continues until
    /// no key is provided and 'wasm-migrate-leases.status=done'.
    MigrateLeases {
        new_code_id: Uint64,
        max_leases: MaxLeases,
    },
    /// Continue a Lease migration
    ///
    /// It migrates the next batch of up to `max_leases` number of Lease instances
    /// and emits the status as specified in `MigrateLeases`.
    MigrateLeasesCont { key: Addr, max_leases: MaxLeases },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    SetupDex(ConnectionParams),
    Config {
        lease_interest_rate_margin: Percent,
        liability: LiabilityDTO,
        lease_interest_payment: InterestPaymentSpec,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Quote {
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
        #[serde(default)]
        max_ltd: Option<Percent>,
    },
    Leases {
        owner: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub config: Config,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
pub struct QuoteResponse {
    pub total: LeaseCoin,
    pub borrow: LpnCoin,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
}
