use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::validate::Validate;

use super::{
    super::{
        impl_mod::{execute_contract, migrate_contract},
        MigrationSpec,
    },
    Protocol, ProtocolContracts,
};

impl ProtocolContracts<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: ProtocolContracts<MigrationSpec>,
    ) {
        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.leaser,
            migration_msgs.leaser,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.lpp,
            migration_msgs.lpp,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.oracle,
            migration_msgs.oracle,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.profit,
            migration_msgs.profit,
        );

        migrate_contract(
            migration_batch,
            post_migration_execute_batch,
            self.reserve,
            migration_msgs.reserve,
        );
    }

    pub(in crate::contracts) fn maybe_migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: ProtocolContracts<Option<MigrationSpec>>,
    ) {
        () = migration_msgs.leaser.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.leaser,
                migration_spec,
            )
        });

        () = migration_msgs.lpp.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.lpp,
                migration_spec,
            )
        });

        () = migration_msgs.oracle.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.oracle,
                migration_spec,
            )
        });

        () = migration_msgs.profit.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.profit,
                migration_spec,
            )
        });

        () = migration_msgs.reserve.map_or((), |migration_spec| {
            migrate_contract(
                migration_batch,
                post_migration_execute_batch,
                self.reserve,
                migration_spec,
            )
        });
    }

    pub(in crate::contracts) fn execute(
        self,
        batch: &mut Batch,
        execute_messages: ProtocolContracts<String>,
    ) {
        execute_contract(batch, self.leaser, execute_messages.leaser);

        execute_contract(batch, self.lpp, execute_messages.lpp);

        execute_contract(batch, self.oracle, execute_messages.oracle);

        execute_contract(batch, self.profit, execute_messages.profit);

        execute_contract(batch, self.reserve, execute_messages.reserve);
    }

    pub(in crate::contracts) fn maybe_execute(
        self,
        batch: &mut Batch,
        execute_messages: ProtocolContracts<Option<String>>,
    ) {
        () = execute_messages.leaser.map_or((), |execute_message| {
            execute_contract(batch, self.leaser, execute_message)
        });

        () = execute_messages.lpp.map_or((), |execute_message| {
            execute_contract(batch, self.lpp, execute_message)
        });

        () = execute_messages.oracle.map_or((), |execute_message| {
            execute_contract(batch, self.oracle, execute_message)
        });

        () = execute_messages.profit.map_or((), |execute_message| {
            execute_contract(batch, self.profit, execute_message)
        });

        () = execute_messages.reserve.map_or((), |execute_message| {
            execute_contract(batch, self.reserve, execute_message)
        });
    }
}

impl<T> Validate for ProtocolContracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.leaser
            .validate(ctx)
            .and_then(|()| self.lpp.validate(ctx))
            .and_then(|()| self.oracle.validate(ctx))
            .and_then(|()| self.profit.validate(ctx))
            .and_then(|()| self.reserve.validate(ctx))
    }
}

impl<T> Validate for Protocol<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    #[inline]
    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.contracts.validate(ctx)
    }
}
