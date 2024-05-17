use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::validate::Validate;

use super::{
    super::{
        impl_mod::{execute_contract, migrate_contract},
        MigrationSpec,
    },
    PlatformContracts,
};

impl PlatformContracts<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: PlatformContracts<MigrationSpec>,
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

    pub(in crate::contracts) fn maybe_migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: PlatformContracts<Option<MigrationSpec>>,
    ) {
        () = migration_msgs.dispatcher.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.dispatcher,
                migration_spec,
            )
        });

        () = migration_msgs.timealarms.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.timealarms,
                migration_spec,
            )
        });

        () = migration_msgs.treasury.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.treasury,
                migration_spec,
            )
        });
    }

    pub(in crate::contracts) fn execute(
        self,
        batch: &mut Batch,
        execute_messages: PlatformContracts<String>,
    ) {
        execute_contract(batch, self.dispatcher, execute_messages.dispatcher);

        execute_contract(batch, self.timealarms, execute_messages.timealarms);

        execute_contract(batch, self.treasury, execute_messages.treasury);
    }

    pub(in crate::contracts) fn maybe_execute(
        self,
        batch: &mut Batch,
        execute_messages: PlatformContracts<Option<String>>,
    ) {
        () = execute_messages.dispatcher.map_or((), |execute_message| {
            execute_contract(batch, self.dispatcher, execute_message)
        });

        () = execute_messages.timealarms.map_or((), |execute_message| {
            execute_contract(batch, self.timealarms, execute_message)
        });

        () = execute_messages.treasury.map_or((), |execute_message| {
            execute_contract(batch, self.treasury, execute_message)
        });
    }
}

impl<T> Validate for PlatformContracts<T>
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
