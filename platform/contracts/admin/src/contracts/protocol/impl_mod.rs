use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{
    contracts::{impl_mod::migrate_contract, MigrationSpec},
    validate::Validate,
};

use super::{Protocol, ProtocolTemplate};

impl ProtocolTemplate<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        migration_msgs: ProtocolTemplate<MigrationSpec>,
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
}

impl<T> Validate for ProtocolTemplate<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> ::std::result::Result<(), Self::Error> {
        self.leaser
            .validate(ctx)
            .and_then(|()| self.lpp.validate(ctx))
            .and_then(|()| self.oracle.validate(ctx))
            .and_then(|()| self.profit.validate(ctx))
            .and_then(|()| self.reserve.validate(ctx))
    }
}

impl Validate for Protocol {
    type Context<'r> = <Addr as Validate>::Context<'r>;

    type Error = <Addr as Validate>::Error;

    #[inline]
    fn validate(&self, ctx: Self::Context<'_>) -> ::std::result::Result<(), Self::Error> {
        self.contracts.validate(ctx)
    }
}
