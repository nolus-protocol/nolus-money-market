use serde::{Deserialize, Serialize};

use admin_contract::msg::{MigrationSpec, ProtocolContracts};
use currency::CurrencyDTO;
use dex::ConnectionParams;
use finance::{duration::Duration, percent::Percent};
use lease::api::{
    DownpaymentCoin, LeaseCoin, LpnCoinDTO, limits::MaxSlippages, open::PositionSpecDTO,
};
use sdk::cosmwasm_std::{Addr, Uint64};
use versioning::ProtocolPackageReleaseId;

use crate::finance::LeaseCurrencies;
pub use crate::state::config::Config;
pub use config::NewConfig;

mod config;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
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
    pub lease_max_slippages: MaxSlippages,
    pub lease_admin: Addr,
    pub dex: ConnectionParams,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

pub type MaxLeases = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    OpenLease {
        currency: CurrencyDTO<LeaseCurrencies>,
        #[serde(default)]
        max_ltd: Option<Percent>,
    },

    /// Configure all lease related parameters
    ///
    /// Only the Lease Admin is permitted to do this
    ConfigLeases(NewConfig),

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
        /// The release ID the new lease code is part of.
        ///
        /// Most of the times this matches the release of the leaser.
        to_release: ProtocolPackageReleaseId,
        max_leases: MaxLeases,
    },

    /// Continue a Lease migration
    ///
    /// It migrates the next batch of up to `max_leases` number of Lease instances
    /// and emits the status as specified in `MigrateLeases`.
    MigrateLeasesCont {
        key: Addr,
        /// The release ID the new lease code is part of.
        ///
        /// Most of the times this matches the release of the leaser.
        /// Provided again on each batch to confirm the release, to avoid its persisting,
        /// and to avoid implementing a complex removal strategy
        to_release: ProtocolPackageReleaseId,
        max_leases: MaxLeases,
    },

    /// Change the lease admin
    ///
    /// Only the current Lease Admin is permitted to do this
    ChangeLeaseAdmin { new: Addr },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {
    Config(NewConfig),

    /// Change the lease admin
    ChangeLeaseAdmin {
        new: Addr,
    },

    CloseProtocol {
        migration_spec: ProtocolContracts<MigrationSpec>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Implementation of [lease::api::authz::AccessCheck::AnomalyResolution]
    CheckAnomalyResolutionPermission {
        by: Addr,
    },
    /// Return [ConfigResponse]
    Config {},
    Leases {
        owner: Addr,
    },
    /// Implementation of [lease::api::limits::PositionLimits::MaxSlippages]
    MaxSlippages {},
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
    Quote {
        downpayment: DownpaymentCoin,
        lease_asset: CurrencyDTO<LeaseCurrencies>,
        max_ltd: Option<Percent>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Clone, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigResponse {
    pub config: Config,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Clone, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct QuoteResponse {
    pub total: LeaseCoin,
    pub borrow: LpnCoinDTO,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use currencies::testing::{LeaseC1, PaymentC1};
    use currency::CurrencyDef;
    use finance::{coin::Coin, duration::Duration, percent::Percent};
    use lease::api::{
        FinalizerExecuteMsg,
        authz::AccessCheck,
        limits::{MaxSlippages, PositionLimits},
        open::PositionSpecDTO,
    };
    use platform::tests as platform_tests;
    use sdk::cosmwasm_std::{Addr, StdError as CwError};
    use serde::Deserialize;

    use crate::{
        msg::{ExecuteMsg, SudoMsg},
        tests,
    };

    use super::QueryMsg;

    #[test]
    fn anomaly_resolution_api_match() {
        let admin = Addr::unchecked("my test admin");
        assert_eq!(
            Ok(AccessCheck::AnomalyResolution { by: admin.clone() }),
            platform_tests::ser_de(&QueryMsg::CheckAnomalyResolutionPermission { by: admin })
                .map_err(|error: CwError| error.to_string()),
        );
    }

    #[test]
    fn finalize_api_match() {
        let customer = Addr::unchecked("c");

        assert_eq!(
            Ok(FinalizerExecuteMsg::FinalizeLease {
                customer: customer.clone()
            }),
            platform_tests::ser_de(&ExecuteMsg::FinalizeLease { customer })
                .map_err(|error: CwError| error.to_string()),
        );
    }

    #[test]
    fn max_slippage_api_match() {
        assert_eq!(
            Ok(PositionLimits::MaxSlippages {}),
            platform_tests::ser_de(&QueryMsg::MaxSlippages {})
                .map_err(|error: CwError| error.to_string()),
        );
    }

    #[test]
    fn release() {
        assert_eq!(
            Ok(QueryMsg::ProtocolPackageRelease {}),
            platform_tests::ser_de(&versioning::query::ProtocolPackage::Release {})
                .map_err(|error: CwError| error.to_string()),
        );

        platform_tests::ser_de::<_, QueryMsg>(&versioning::query::PlatformPackage::Release {})
            .unwrap_err();
    }

    #[test]
    fn new_config_is_transparrent() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub enum ConfigInlineInSudoMsg {
            Config {
                lease_interest_rate_margin: Percent,
                lease_position_spec: PositionSpecDTO,
                lease_due_period: Duration,
                lease_max_slippages: MaxSlippages,
            },
        }

        let new_config = tests::new_config();

        assert_eq!(
            Ok(ConfigInlineInSudoMsg::Config {
                lease_interest_rate_margin: new_config.lease_interest_rate_margin,
                lease_position_spec: new_config.lease_position_spec,
                lease_due_period: new_config.lease_due_period,
                lease_max_slippages: new_config.lease_max_slippages
            }),
            platform_tests::ser_de(&SudoMsg::Config(new_config))
                .map_err(|error: CwError| error.to_string()),
        );
    }

    #[test]
    fn no_max_ltd() {
        let quote_bin = "{\"quote\":{\"downpayment\":{\"amount\":\"10\",\"ticker\":\"NLS\"},\"lease_asset\":\"LC1\"}}";

        assert_eq!(
            QueryMsg::Quote {
                downpayment: Coin::<PaymentC1>::new(10).into(),
                lease_asset: *LeaseC1::dto(),
                max_ltd: None,
            },
            cosmwasm_std::from_json(quote_bin).expect("deserialization failed"),
        );
    }
}
