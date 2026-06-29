use serde::{Deserialize, Serialize};

use platform::contract::Code;
use sdk::{cosmwasm_std::testing, cw_storage_plus::Item, testing as sdk_testing};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    package_name, package_version,
};

use crate::{api::MigrateMsg, contract::migrate, error::Error};

use super::{
    CONNECTION_ID, CONTRACT_STORAGE_VERSION, DEX_LABEL, PROFIT_CODE_ID, PROFIT_CONTRACT,
    TRANSFER_CHANNEL, deps, instantiate_default,
};

#[test]
fn migrate_same_release_succeeds() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap();
    assert_eq!(0, res.messages.len());
}

// Overwrites the stored config with the shape that predates the required
// `profit_contract` field (the singleton callback target) — the probe in
// `migrate` must refuse the upgrade instead of letting the instance brick on
// the first post-upgrade load.
#[test]
fn migrate_with_pre_profit_contract_config_rejected() {
    #[derive(Serialize, Deserialize)]
    struct LegacyConfig {
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        profit_code: Code,
    }
    const LEGACY_STORAGE: Item<LegacyConfig> = Item::new("config");

    let mut deps = deps();
    instantiate_default(deps.as_mut());
    LEGACY_STORAGE
        .save(
            deps.as_mut().storage,
            &LegacyConfig {
                connection_id: CONNECTION_ID.into(),
                dex_label: DEX_LABEL.into(),
                transfer_channel: TRANSFER_CHANNEL.into(),
                profit_code: Code::unchecked(PROFIT_CODE_ID),
            },
        )
        .unwrap();

    let err = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap_err();
    assert!(
        matches!(err, Error::IncompatibleStoredConfig(_)),
        "got {err:?}"
    );
}

// Overwrites the stored config with a current-shape value the current code
// would never have accepted (non-canonical transfer channel) — the probe must
// refuse the upgrade on the invariant, not just on deserialization.
#[test]
fn migrate_with_invariant_violating_config_rejected() {
    #[derive(Serialize, Deserialize)]
    struct RawConfig {
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        profit_code: Code,
        profit_contract: sdk::cosmwasm_std::Addr,
    }
    const RAW_STORAGE: Item<RawConfig> = Item::new("config");

    let mut deps = deps();
    instantiate_default(deps.as_mut());
    RAW_STORAGE
        .save(
            deps.as_mut().storage,
            &RawConfig {
                connection_id: CONNECTION_ID.into(),
                dex_label: DEX_LABEL.into(),
                transfer_channel: "channel-007".into(),
                profit_code: Code::unchecked(PROFIT_CODE_ID),
                profit_contract: sdk_testing::user(PROFIT_CONTRACT),
            },
        )
        .unwrap();

    let err = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap_err();
    assert!(matches!(err, Error::MalformedStoredConfig), "got {err:?}");
}

#[test]
fn migrate_mismatched_to_release_id_propagates_update_software_error() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    let mut msg = migrate_msg();
    msg.to_release = ProtocolPackageReleaseId::new(
        ReleaseId::new_test("not-the-build-id"),
        ReleaseId::new_test("not-the-build-id"),
    );
    let err = migrate(deps.as_mut(), testing::mock_env(), msg).unwrap_err();
    assert!(matches!(err, Error::UpdateSoftware(_)), "got {err:?}");
}

fn migrate_msg() -> ProtocolMigrationMessage<MigrateMsg> {
    // Both env vars are supplied by `protocol/.cargo/config.toml` (see RUNBOOK
    // entry "Cargo / cargo config override entry for SOFTWARE_RELEASE_ID").
    // Running `cargo test` from outside the protocol workspace will fail to
    // compile this file with a missing-env-var error.
    const SOFTWARE_ID: &str = env!("SOFTWARE_RELEASE_ID");
    const PROTOCOL_ID: &str = env!("PROTOCOL_RELEASE_ID");
    let release = ProtocolPackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );
    ProtocolMigrationMessage {
        migrate_from: release,
        to_release: ProtocolPackageReleaseId::new(
            ReleaseId::new_test(SOFTWARE_ID),
            ReleaseId::new_test(PROTOCOL_ID),
        ),
        message: MigrateMsg {},
    }
}
