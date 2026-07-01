use serde::{Deserialize, Serialize};

use dex::ConnectionParams;
use platform::contract::external::Code;
use remote_profit::callback::RemoteProfitCallback;
use sdk::cosmwasm_std::Addr;

use crate::CadenceHours;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub cadence_hours: CadenceHours,
    pub treasury: Addr,
    pub oracle: Addr,
    pub timealarms: Addr,
    /// The funding/draining ICS-20 transfer channel and connection. No ICA is
    /// registered — funding rides this channel directly (`Account::funding`).
    pub dex: ConnectionParams,
    /// The remote-profit controller authorised to deliver `RemoteProfitCallback`.
    pub remote_profit_controller: Addr,
    /// Code id of the `drain_vault` the profit instantiates and drains into.
    pub vault_code_id: Code,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    TimeAlarm {},
    Config {
        cadence_hours: CadenceHours,
    },

    /// An entry point for safe delivery of a Dex response
    ///
    /// Invoked always by the same contract instance.
    DexCallback(),

    /// Delivery of a remote-profit controller callback resolving an in-flight
    /// remote leg (swap or transfer-out). Authorised against
    /// `Config.remote_profit_controller`. Must byte-match the controller shim
    /// `remote_profit/src/profit_callback.rs`.
    RemoteProfitCallback(RemoteProfitCallback),

    /// Heal the profit past a middleware failure
    Heal(),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigResponse {
    pub cadence_hours: CadenceHours,
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use platform::tests as platform_tests;
    use remote_profit::callback::{RemoteOperationOutcome, RemoteProfitCallback};
    use sdk::cosmwasm_std::from_json;

    use super::{ExecuteMsg, QueryMsg};

    #[test]
    fn release() {
        assert_eq!(
            QueryMsg::ProtocolPackageRelease {},
            platform_tests::ser_de::<_, QueryMsg>(&versioning::query::ProtocolPackage::Release {})
                .unwrap(),
        );
    }

    /// C-CB4 cross-crate byte-match: the exact bytes the controller pins
    /// (`remote_profit/src/ibc/tests/packets.rs` `dispatched_callback_wire_shape_pinned`)
    /// must decode into profit's real `ExecuteMsg::RemoteProfitCallback` variant.
    #[test]
    fn controller_callback_bytes_decode_into_the_execute_arm() {
        const PINNED: &[u8] =
            br#"{"remote_profit_callback":{"nonce":0,"outcome":"operation_timeout"}}"#;

        assert_eq!(
            ExecuteMsg::RemoteProfitCallback(RemoteProfitCallback {
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            }),
            from_json::<ExecuteMsg>(PINNED).expect("the pinned controller bytes decode"),
        );
    }

    /// The dropped `DexCallbackContinue` arm must not decode — proving the ICA
    /// continue variant is gone from the wire.
    #[test]
    fn dropped_dex_callback_continue_is_rejected() {
        assert!(from_json::<ExecuteMsg>(br#"{"dex_callback_continue":[]}"#).is_err());
    }
}
