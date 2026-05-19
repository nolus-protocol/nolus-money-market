use sdk::cosmwasm_std::testing;
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    package_name, package_version,
};

use crate::{api::MigrateMsg, contract::migrate, error::Error};

use super::{CONTRACT_STORAGE_VERSION, deps, instantiate_default};

#[test]
fn migrate_same_release_succeeds() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap();
    assert_eq!(0, res.messages.len());
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
