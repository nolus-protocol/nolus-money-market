use serde::{Deserialize, Serialize};

#[cfg(feature = "contract")]
use platform::batch::Batch;
#[cfg(feature = "contract")]
use sdk::cosmwasm_std::Addr;
use sdk::schemars::{self, JsonSchema};

#[cfg(feature = "contract")]
use crate::validate::Validate;

#[cfg(feature = "contract")]
use super::{maybe_execute_contract, maybe_migrate_contract, MigrationSpec};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Platform<T> {
    pub dispatcher: T,
    pub timealarms: T,
    pub treasury: T,
}

#[cfg(feature = "contract")]
impl Platform<Addr> {
    pub(super) fn migrate(self, batch: &mut Batch, migration_msgs: MigrationSpecs) {
        maybe_migrate_contract(batch, self.dispatcher, migration_msgs.dispatcher);
        maybe_migrate_contract(batch, self.timealarms, migration_msgs.timealarms);
        maybe_migrate_contract(batch, self.treasury, migration_msgs.treasury);
    }

    pub(super) fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: PostMigrationExecutes,
    ) {
        maybe_execute_contract(batch, self.dispatcher, execution_msgs.dispatcher);
        maybe_execute_contract(batch, self.timealarms, execution_msgs.timealarms);
        maybe_execute_contract(batch, self.treasury, execution_msgs.treasury);
    }
}

#[cfg(feature = "contract")]
impl<T> Validate for Platform<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.dispatcher.validate(ctx)?;

        self.timealarms.validate(ctx)?;

        self.treasury.validate(ctx)
    }
}

#[cfg(feature = "contract")]
type MigrationSpecs = Platform<Option<MigrationSpec>>;

#[cfg(feature = "contract")]
type PostMigrationExecutes = Platform<Option<String>>;
