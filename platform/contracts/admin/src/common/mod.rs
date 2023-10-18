use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use platform::{batch::Batch, contract::CodeId};
use sdk::{
    cosmwasm_std::{Addr, Binary, WasmMsg},
    schemars::{self, JsonSchema},
};

use crate::{
    common::type_defs::{
        Contracts, ContractsMigration, ContractsPostMigrationExecute, MaybeMigrateContract,
    },
    ContractError, ContractResult,
};

use self::type_defs::{
    PlatformContracts, PlatformContractsMigration, PlatformContractsPostMigrationExecute,
    ProtocolContracts, ProtocolContractsMigration, ProtocolContractsPostMigrationExecute,
};
pub(crate) use self::{
    checked::{Addr as CheckedAddr, StoredAddr},
    transform::Transform,
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

impl<M> Transform for MigrationSpec<M>
where
    M: Transform,
{
    type Context<'r> = M::Context<'r>;

    type Output = MigrationSpec<M::Output>;

    type Error = M::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        self.migrate_msg
            .transform(ctx)
            .map(|migrate_msg: M::Output| MigrationSpec {
                code_id: self.code_id,
                migrate_msg,
            })
    }
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
pub struct ContractsTemplate<T> {
    pub platform: Platform<T>,
    pub protocol: BTreeMap<String, Protocol<T>>,
}

impl<T> Transform for ContractsTemplate<T>
where
    T: Transform,
{
    type Context<'r> = T::Context<'r>;

    type Output = ContractsTemplate<T::Output>;

    type Error = T::Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output {
            platform: self.platform.transform(ctx)?,
            protocol: self
                .protocol
                .into_iter()
                .map(|(key, value): (String, Protocol<T>)| {
                    value
                        .transform(ctx)
                        .map(|value: Protocol<T::Output>| (key, value))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl Contracts {
    pub(crate) fn migrate(self, mut migration_msgs: ContractsMigration) -> ContractResult<Batch> {
        let mut batch: Batch = Batch::default();

        self.platform.migrate(&mut batch, migration_msgs.platform);

        self.protocol
            .into_iter()
            .try_for_each(|(dex, protocol): (String, ProtocolContracts)| {
                migration_msgs
                    .protocol
                    .remove(&dex)
                    .map(|migration_msgs: Protocol<MaybeMigrateContract>| {
                        protocol.migrate(&mut batch, migration_msgs)
                    })
                    .ok_or(ContractError::MissingDex(dex))
            })
            .and_then(|()| {
                if let Some((dex, _)) = migration_msgs.protocol.pop_first() {
                    Err(ContractError::UnknownDex(dex))
                } else {
                    Ok(batch)
                }
            })
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
            .try_for_each(|(dex, protocol): (String, ProtocolContracts)| {
                execution_msgs
                    .protocol
                    .remove(&dex)
                    .map(|execution_msgs: Protocol<Option<String>>| {
                        protocol.post_migration_execute(&mut batch, execution_msgs)
                    })
                    .ok_or(ContractError::MissingDex(dex))
            })
            .and_then(|()| {
                if let Some((dex, _)) = execution_msgs.protocol.pop_first() {
                    Err(ContractError::UnknownDex(dex))
                } else {
                    Ok(batch)
                }
            })
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

impl ProtocolContracts {
    fn migrate(self, batch: &mut Batch, migration_msgs: ProtocolContractsMigration) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);
        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);
        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);
        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);
    }

    fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: ProtocolContractsPostMigrationExecute,
    ) {
        maybe_execute_contract(batch, self.leaser, execution_msgs.leaser);
        maybe_execute_contract(batch, self.lpp, execution_msgs.lpp);
        maybe_execute_contract(batch, self.oracle, execution_msgs.oracle);
        maybe_execute_contract(batch, self.profit, execution_msgs.profit);
    }
}
