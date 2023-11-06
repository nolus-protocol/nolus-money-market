use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use platform::{batch::Batch, contract::CodeId};
use sdk::{
    cosmwasm_std::{Addr, Binary, WasmMsg},
    schemars::{self, JsonSchema},
};

use crate::{
    common::type_defs::{ContractsMigration, ContractsPostMigrationExecute, MaybeMigrateContract},
    ContractError, ContractResult,
};

pub(crate) use self::{
    checked::{Addr as CheckedAddr, StoredAddr},
    transform::Transform,
};
use self::{
    transform::TransformByValue,
    type_defs::{
        ContractsGroupedByDex, PlatformContracts, PlatformContractsMigration,
        PlatformContractsPostMigrationExecute,
    },
};

mod checked;
mod transform;
pub(crate) mod type_defs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrationSpec<M> {
    pub code_id: CodeId,
    pub migrate_msg: M,
}

pub(crate) fn maybe_migrate_contract(
    batch: &mut Batch,
    addr: CheckedAddr,
    migrate: MaybeMigrateContract,
) {
    if let Some(migrate) = migrate {
        batch.schedule_execute_on_success_reply(
            WasmMsg::Migrate {
                contract_addr: Addr::from(addr).into_string(),
                new_code_id: migrate.code_id,
                msg: Binary(migrate.migrate_msg.into()),
            },
            0,
        );
    }
}

pub(crate) fn maybe_execute_contract(
    batch: &mut Batch,
    addr: CheckedAddr,
    execute: Option<String>,
) {
    if let Some(execute) = execute {
        batch.schedule_execute_no_reply(WasmMsg::Execute {
            contract_addr: Addr::from(addr).into_string(),
            msg: Binary(execute.into()),
            funds: vec![],
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContractsTemplate<T, U = Protocol<BTreeMap<String, T>>> {
    pub platform: Platform<T>,
    pub protocol: U,
}

impl<T> Transform for ContractsTemplate<T, BTreeMap<String, Protocol<T>>>
where
    T: Transform,
{
    type Context<'r> = T::Context<'r>;

    type Output = ContractsTemplate<T::Output, BTreeMap<String, Protocol<T::Output>>>;

    type Error = T::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output {
            platform: self.platform.transform(ctx)?,
            protocol: TransformByValue::new(self.protocol).transform(ctx)?,
        })
    }
}

impl ContractsGroupedByDex {
    pub(crate) fn migrate(self, mut migration_msgs: ContractsMigration) -> ContractResult<Batch> {
        let mut batch: Batch = Batch::default();

        self.platform.migrate(&mut batch, migration_msgs.platform);

        self.protocol
            .into_iter()
            .try_for_each(|(dex, protocol): (String, Protocol<CheckedAddr>)| {
                migration_msgs.protocol.extract_entry(dex).map(
                    |migration_msgs: Protocol<MaybeMigrateContract>| {
                        protocol.migrate(&mut batch, migration_msgs)
                    },
                )
            })
            .and_then(|()| migration_msgs.protocol.ensure_empty())
            .map(|()| batch)
    }

    pub(crate) fn post_migration_execute(
        self,
        mut execution_msgs: ContractsPostMigrationExecute,
    ) -> ContractResult<Batch> {
        let mut batch: Batch = Batch::default();

        self.platform
            .post_migration_execute(&mut batch, execution_msgs.platform);

        self.protocol
            .into_iter()
            .try_for_each(|(dex, protocol): (String, Protocol<CheckedAddr>)| {
                execution_msgs.protocol.extract_entry(dex).map(
                    |execution_msgs: Protocol<Option<String>>| {
                        protocol.post_migration_execute(&mut batch, execution_msgs)
                    },
                )
            })
            .and_then(|()| execution_msgs.protocol.ensure_empty())
            .map(|()| batch)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Platform<T> {
    pub dispatcher: T,
    pub timealarms: T,
    pub treasury: T,
}

impl<T> Transform for Platform<T>
where
    T: Transform,
{
    type Context<'r> = T::Context<'r>;

    type Output = Platform<T::Output>;

    type Error = T::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(Platform {
            dispatcher: self.dispatcher.transform(ctx)?,
            timealarms: self.timealarms.transform(ctx)?,
            treasury: self.treasury.transform(ctx)?,
        })
    }
}

impl PlatformContracts {
    fn migrate(self, batch: &mut Batch, migration_msgs: PlatformContractsMigration) {
        maybe_migrate_contract(batch, self.dispatcher, migration_msgs.dispatcher);
        maybe_migrate_contract(batch, self.timealarms, migration_msgs.timealarms);
        maybe_migrate_contract(batch, self.treasury, migration_msgs.treasury);
    }

    fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: PlatformContractsPostMigrationExecute,
    ) {
        maybe_execute_contract(batch, self.dispatcher, execution_msgs.dispatcher);
        maybe_execute_contract(batch, self.timealarms, execution_msgs.timealarms);
        maybe_execute_contract(batch, self.treasury, execution_msgs.treasury);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Protocol<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
}

impl Protocol<CheckedAddr> {
    fn migrate(self, batch: &mut Batch, migration_msgs: Protocol<MaybeMigrateContract>) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);
    }

    fn post_migration_execute(self, batch: &mut Batch, migration_msgs: Protocol<Option<String>>) {
        maybe_execute_contract(batch, self.leaser, migration_msgs.leaser);

        maybe_execute_contract(batch, self.lpp, migration_msgs.lpp);

        maybe_execute_contract(batch, self.oracle, migration_msgs.oracle);

        maybe_execute_contract(batch, self.profit, migration_msgs.profit);
    }
}

impl<T> Protocol<BTreeMap<String, T>> {
    fn extract_entry(&mut self, dex: String) -> ContractResult<Protocol<T>> {
        if let Some((leaser, lpp, oracle, profit)) =
            self.leaser.remove(&dex).and_then(|leaser: T| {
                self.lpp.remove(&dex).and_then(|lpp: T| {
                    self.oracle.remove(&dex).and_then(|oracle: T| {
                        self.profit
                            .remove(&dex)
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
            Err(ContractError::MissingDex(dex))
        }
    }

    fn ensure_empty(self) -> ContractResult<()> {
        [self.leaser, self.lpp, self.oracle, self.profit]
            .into_iter()
            .try_for_each(|mut map: BTreeMap<String, T>| {
                if let Some((dex, _)) = map.pop_last() {
                    Err(ContractError::MissingDex(dex))
                } else {
                    Ok(())
                }
            })
    }
}

impl<T> Transform for Protocol<T>
where
    T: Transform,
{
    type Context<'r> = T::Context<'r>;

    type Output = Protocol<T::Output>;

    type Error = T::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(Protocol {
            leaser: self.leaser.transform(ctx)?,
            lpp: self.lpp.transform(ctx)?,
            oracle: self.oracle.transform(ctx)?,
            profit: self.profit.transform(ctx)?,
        })
    }
}
