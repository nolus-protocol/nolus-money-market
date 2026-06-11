use serde::{Deserialize, Serialize};

use platform::contract::Code;
use sdk::{cosmwasm_std::testing, cw_storage_plus::Item};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    package_name, package_version,
};

use crate::{api::MigrateMsg, contract::migrate, error::Error};

use super::{
    CONNECTION_ID, CONTRACT_STORAGE_VERSION, DEX_LABEL, LEASE_CODE_ID, deps, instantiate_default,
};

#[test]
fn migrate_same_release_succeeds() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap();
    assert_eq!(0, res.messages.len());
}

// Overwrites the stored config with the shape that predates the required
// `transfer_channel` field — the probe in `migrate` must refuse the upgrade
// instead of letting the instance brick on the first post-upgrade load.
#[test]
fn migrate_with_pre_transfer_channel_config_rejected() {
    #[derive(Serialize, Deserialize)]
    struct LegacyConfig {
        connection_id: String,
        dex_label: String,
        lease_code: Code,
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
                lease_code: Code::unchecked(LEASE_CODE_ID),
            },
        )
        .unwrap();

    let err = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap_err();
    assert!(
        matches!(err, Error::IncompatibleStoredConfig(_)),
        "got {err:?}"
    );
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
