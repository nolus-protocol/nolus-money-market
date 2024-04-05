use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{
    contracts::{impl_mod::migrate_contract, MigrationSpec},
    validate::Validate,
};

use super::PlatformTemplate;

impl PlatformTemplate<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: PlatformTemplate<MigrationSpec>,
    ) {
        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.dispatcher,
            migration_msgs.dispatcher,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.timealarms,
            migration_msgs.timealarms,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.treasury,
            migration_msgs.treasury,
        );
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
