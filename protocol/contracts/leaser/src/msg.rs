use serde::{Deserialize, Serialize};

use admin_contract::msg::{MigrationSpec, ProtocolContracts};
use currency::CurrencyDTO;
use finance::{duration::Duration, percent::Percent};
use lease::api::{
    open::{ConnectionParams, PositionSpecDTO},
    DownpaymentCoin, LeaseCoin, LpnCoinDTO,
};
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    schemars::{self, JsonSchema},
};

use crate::finance::LeaseCurrencies;
pub use crate::state::config::Config;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub lease_code: Uint64,
    pub lpp: Addr,
    pub profit: Addr,
    pub reserve: Addr,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub protocols_registry: Addr,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_interest_rate_margin: Percent,
    pub lease_due_period: Duration,
    pub dex: ConnectionParams,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

pub type MaxLeases = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    OpenLease {
        currency: CurrencyDTO<LeaseCurrencies>,
        #[serde(default)]
        max_ltd: Option<Percent>,
    },
    /// A callback from a lease that it has just entered a final state
    ///
    /// It matches the `lease::api::FinalizerExecuteMsg::FinalizeLease`.
    FinalizeLease { customer: Addr },
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
        // Since this is an external system API we should not use [Code].
        new_code_id: Uint64,
        max_leases: MaxLeases,
    },
    /// Continue a Lease migration
    ///
    /// It migrates the next batch of up to `max_leases` number of Lease instances
    /// and emits the status as specified in `MigrateLeases`.
    MigrateLeasesCont { key: Addr, max_leases: MaxLeases },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {
    Config {
        lease_interest_rate_margin: Percent,
        lease_position_spec: PositionSpecDTO,
        lease_due_period: Duration,
    },
    CloseProtocol {
        // Since this is an external system API we should not use [Code].
        new_lease_code_id: Uint64,
        migration_spec: ProtocolContracts<MigrationSpec>,
        /// `ForceClose::KillProtocol` closes the protocol even if it has not closed leases
        /// by migrating them to void.
        ///
        /// Limitation!
        /// The leases number is limited up to the max gas.
        #[serde(default)]
        force: ForceClose,
    },
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ForceClose {
    #[default]
    No,
    KillProtocol,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Quote {
        downpayment: DownpaymentCoin,
        lease_asset: CurrencyDTO<LeaseCurrencies>,
        // TODO get rid of the default-ness
        #[serde(default)]
        max_ltd: Option<Percent>,
    },
    Leases {
        owner: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigResponse {
    pub config: Config,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct QuoteResponse {
    pub total: LeaseCoin,
    pub borrow: LpnCoinDTO,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
}

#[cfg(test)]
mod test {
    use crate::msg::ExecuteMsg;
    use lease::api::FinalizerExecuteMsg;
    use sdk::cosmwasm_std::Addr;

    #[test]
    fn finalize_api_match() {
        use sdk::cosmwasm_std::{from_json, to_json_vec};

        let customer = Addr::unchecked("c");
        let finalize_bin = to_json_vec(&ExecuteMsg::FinalizeLease {
            customer: customer.clone(),
        })
        .expect("serialization passed");
        let msg_out: FinalizerExecuteMsg = from_json(finalize_bin).expect("deserialization passed");
        assert_eq!(FinalizerExecuteMsg::FinalizeLease { customer }, msg_out);
    }
}
