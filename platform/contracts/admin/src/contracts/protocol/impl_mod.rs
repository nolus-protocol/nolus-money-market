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

use super::Protocol;

impl Protocol<Addr> {
    pub(in crate::contracts) fn migrate(
        self,
        batch: &mut Batch,
        migration_msgs: ProtocolMigrationSpec,
    ) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);
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
    }
}

impl<T> Protocol<BTreeMap<String, T>> {
    pub(in crate::contracts) fn extract_entry(&mut self, protocol: String) -> Result<Protocol<T>> {
        if let Some((leaser, lpp, oracle, profit)) =
            self.leaser.remove(&protocol).and_then(|leaser: T| {
                self.lpp.remove(&protocol).and_then(|lpp: T| {
                    self.oracle.remove(&protocol).and_then(|oracle: T| {
                        self.profit
                            .remove(&protocol)
                            .map(|profit: T| (leaser, lpp, oracle, profit))
                    })
                })
            })
        {
            Ok(Protocol {
                leaser,
                lpp,
                oracle,
                profit,
            })
        } else {
            Err(Error::MissingProtocol(protocol))
        }
    }

    pub(in crate::contracts) fn ensure_empty(self) -> Result<()> {
        [self.leaser, self.lpp, self.oracle, self.profit]
            .into_iter()
            .try_for_each(|mut map: BTreeMap<String, T>| {
                if let Some((protocol, _)) = map.pop_last() {
                    Err(Error::MissingProtocol(protocol))
                } else {
                    Ok(())
                }
            })
    }
}

impl<T> Validate for Protocol<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> ::std::result::Result<(), Self::Error> {
        self.leaser.validate(ctx)?;

        self.lpp.validate(ctx)?;

        self.oracle.validate(ctx)?;

        self.profit.validate(ctx)
    }
}

type ProtocolMigrationSpec = Protocol<Option<MigrationSpec>>;

type ProtocolPostMigrationExecute = Protocol<Option<String>>;
