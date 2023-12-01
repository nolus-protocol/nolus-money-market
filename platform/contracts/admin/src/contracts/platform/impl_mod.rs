use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{
    contracts::{
        impl_mod::{maybe_execute_contract, maybe_migrate_contract},
        MigrationSpec,
    },
    validate::Validate,
};

use super::PlatformTemplate;

impl PlatformTemplate<Addr> {
    pub(in crate::contracts) fn migrate(self, batch: &mut Batch, migration_msgs: MigrationSpecs) {
        maybe_migrate_contract(batch, self.dispatcher, migration_msgs.dispatcher);
        maybe_migrate_contract(batch, self.timealarms, migration_msgs.timealarms);
        maybe_migrate_contract(batch, self.treasury, migration_msgs.treasury);
    }

    pub(in crate::contracts) fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: PostMigrationExecutes,
    ) {
        maybe_execute_contract(batch, self.dispatcher, execution_msgs.dispatcher);
        maybe_execute_contract(batch, self.timealarms, execution_msgs.timealarms);
        maybe_execute_contract(batch, self.treasury, execution_msgs.treasury);
    }
}

impl<T> Validate for PlatformTemplate<T>
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

type MigrationSpecs = PlatformTemplate<Option<MigrationSpec>>;

type PostMigrationExecutes = PlatformTemplate<Option<String>>;
