#[cfg(feature = "contract")]
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "contract")]
use platform::batch::Batch;
#[cfg(feature = "contract")]
use sdk::cosmwasm_std::Addr;
use sdk::schemars::{self, JsonSchema};

#[cfg(feature = "contract")]
use crate::{error::Error, result::Result, validate::Validate};

#[cfg(feature = "contract")]
use super::{maybe_execute_contract, maybe_migrate_contract, MigrationSpec};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Protocol<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
}

#[cfg(feature = "contract")]
impl Protocol<Addr> {
    pub(super) fn migrate(self, batch: &mut Batch, migration_msgs: ProtocolMigrationSpec) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);
    }

    pub(super) fn post_migration_execute(
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

#[cfg(feature = "contract")]
impl<T> Protocol<BTreeMap<String, T>> {
    pub(super) fn extract_entry(&mut self, protocol: String) -> Result<Protocol<T>> {
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

    pub(super) fn ensure_empty(self) -> Result<()> {
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

#[cfg(feature = "contract")]
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

#[cfg(feature = "contract")]
type ProtocolMigrationSpec = Protocol<Option<MigrationSpec>>;

#[cfg(feature = "contract")]
type ProtocolPostMigrationExecute = Protocol<Option<String>>;
