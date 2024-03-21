use std::collections::BTreeMap;

use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{
    contracts::{
        impl_mod::{maybe_execute_contract, maybe_migrate_contract},
        MigrationSpec,
    },
    error::Error,
    result::Result,
    validate::Validate,
};

use super::{Protocol, ProtocolTemplate};

impl ProtocolTemplate<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        batch: &mut Batch,
        migration_msgs: ProtocolMigrationSpec,
    ) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);

        maybe_migrate_contract(batch, self.reserve, migration_msgs.reserve);
    }

    pub(in crate::contracts) fn post_migration_execute(
        self,
        batch: &mut Batch,
        migration_msgs: ProtocolPostMigrationExecute,
    ) {
        maybe_execute_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_execute_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_execute_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_execute_contract(batch, self.profit, migration_msgs.profit);

        maybe_execute_contract(batch, self.reserve, migration_msgs.reserve);
    }
}

impl<T> ProtocolTemplate<BTreeMap<String, T>> {
    pub(in crate::contracts) fn extract_entry(
        &mut self,
        protocol: String,
    ) -> Result<ProtocolTemplate<T>> {
        self.leaser
            .remove(&protocol)
            .and_then(|leaser| self.lpp.remove(&protocol).map(|lpp| (leaser, lpp)))
            .and_then(|contracts| {
                self.oracle
                    .remove(&protocol)
                    .map(|oracle| (contracts, oracle))
            })
            .and_then(|contracts| {
                self.profit
                    .remove(&protocol)
                    .map(|profit| (contracts, profit))
            })
            .and_then(|contracts| {
                self.reserve
                    .remove(&protocol)
                    .map(|reserve| (contracts, reserve))
            })
            .map_or_else(
                || Err(Error::MissingProtocol(protocol)),
                |((((leaser, lpp), oracle), profit), reserve)| {
                    Ok(ProtocolTemplate {
                        leaser,
                        lpp,
                        oracle,
                        profit,
                        reserve,
                    })
                },
            )
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

type ProtocolMigrationSpec = ProtocolTemplate<Option<MigrationSpec>>;

type ProtocolPostMigrationExecute = ProtocolTemplate<Option<String>>;
