use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

use super::UpdatablePackage;

#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "Package: Serialize,
            Package::ReleaseId: Serialize,
            ContractMsg: Serialize",
        deserialize = "Package: Deserialize<'de>,
            Package::ReleaseId: Deserialize<'de>,
            ContractMsg: Deserialize<'de>",
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct MigrationMessage<Package, ContractMsg>
where
    Package: UpdatablePackage,
{
    pub migrate_from: Package,
    pub to_release: Package::ReleaseId,
    pub message: ContractMsg,
}

impl<Package, ContractMsg> MigrationMessage<Package, ContractMsg>
where
    Package: UpdatablePackage,
{
    pub const fn new(
        migrate_from: Package,
        to_release: Package::ReleaseId,
        message: ContractMsg,
    ) -> Self {
        Self {
            migrate_from,
            to_release,
            message,
        }
    }
}

impl<Package, ContractMsg> Debug for MigrationMessage<Package, ContractMsg>
where
    Package: UpdatablePackage + Debug,
    Package::ReleaseId: Debug,
    ContractMsg: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MigrationMessage")
            .field("migrate_from", &self.migrate_from)
            .field("to_release", &self.to_release)
            .field("message", &self.message)
            .finish()
    }
}
