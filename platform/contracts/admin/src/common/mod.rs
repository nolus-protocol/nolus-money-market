use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use platform::{batch::Batch, contract::CodeId};
use sdk::{
    cosmwasm_std::{Addr, Binary, QuerierWrapper, WasmMsg},
    schemars::{self, JsonSchema},
};

use crate::{
    common::type_defs::{
        Contracts, ContractsMigration, ContractsPostMigrationExecute, MaybeMigrateContract,
    },
    ContractError, ContractResult,
};

use self::type_defs::{
    DexBoundContracts, DexBoundContractsMigration, DexBoundContractsPostMigrationExecute,
    DexIndependentContracts, DexIndependentContractsMigration,
    DexIndependentContractsPostMigrationExecute,
};

pub(crate) mod type_defs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrationSpec<M> {
    pub code_id: CodeId,
    pub migrate_msg: M,
}

pub fn maybe_migrate_contract(batch: &mut Batch, addr: Addr, migrate: MaybeMigrateContract) {
    if let Some(migrate) = migrate {
        batch.schedule_execute_on_success_reply(
            WasmMsg::Migrate {
                contract_addr: addr.into_string(),
                new_code_id: migrate.code_id,
                msg: Binary(migrate.migrate_msg.into()),
            },
            0,
        );
    }
}

pub fn maybe_execute_contract(batch: &mut Batch, addr: Addr, execute: Option<String>) {
    if let Some(execute) = execute {
        batch.schedule_execute_no_reply(WasmMsg::Execute {
            contract_addr: addr.into_string(),
            msg: Binary(execute.into()),
            funds: vec![],
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContractsTemplate<T> {
    pub dex_independent: DexIndependent<T>,
    pub dex_bound: BTreeMap<String, DexBound<T>>,
}

impl Contracts {
    pub(crate) fn validate(&self, querier: &QuerierWrapper<'_>) -> ContractResult<()> {
        self.dex_independent
            .validate(querier)
            .and_then(|()| {
                self.dex_bound
                    .values()
                    .try_for_each(|dex_bound: &DexBound<Addr>| dex_bound.validate(querier))
            })
            .map_err(Into::into)
    }

    pub(crate) fn migrate(self, mut migration_msgs: ContractsMigration) -> ContractResult<Batch> {
        let mut batch: Batch = Batch::default();

        self.dex_independent
            .migrate(&mut batch, migration_msgs.dex_independent);

        self.dex_bound
            .into_iter()
            .try_for_each(|(dex, dex_bound): (String, DexBound<Addr>)| {
                migration_msgs
                    .dex_bound
                    .remove(&dex)
                    .map(|migration_msgs: DexBound<MaybeMigrateContract>| {
                        dex_bound.migrate(&mut batch, migration_msgs)
                    })
                    .ok_or(ContractError::MissingDex(dex))
            })
            .and_then(|()| {
                if let Some((dex, _)) = migration_msgs.dex_bound.pop_first() {
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

        self.dex_independent
            .post_migration_execute(&mut batch, execution_msgs.dex_independent);

        self.dex_bound
            .into_iter()
            .try_for_each(|(dex, dex_bound): (String, DexBound<Addr>)| {
                execution_msgs
                    .dex_bound
                    .remove(&dex)
                    .map(|execution_msgs: DexBound<Option<String>>| {
                        dex_bound.post_migration_execute(&mut batch, execution_msgs)
                    })
                    .ok_or(ContractError::MissingDex(dex))
            })
            .and_then(|()| {
                if let Some((dex, _)) = execution_msgs.dex_bound.pop_first() {
                    Err(ContractError::UnknownDex(dex))
                } else {
                    Ok(batch)
                }
            })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DexIndependent<T> {
    pub dispatcher: T,
    pub timealarms: T,
    pub treasury: T,
}

impl DexIndependentContracts {
    fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), platform::error::Error> {
        platform::contract::validate_addr(querier, &self.dispatcher)
            .and_then(|()| platform::contract::validate_addr(querier, &self.timealarms))
            .and_then(|()| platform::contract::validate_addr(querier, &self.treasury))
    }

    fn migrate(self, batch: &mut Batch, migration_msgs: DexIndependentContractsMigration) {
        maybe_migrate_contract(batch, self.dispatcher, migration_msgs.dispatcher);
        maybe_migrate_contract(batch, self.timealarms, migration_msgs.timealarms);
        maybe_migrate_contract(batch, self.treasury, migration_msgs.treasury);
    }

    fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: DexIndependentContractsPostMigrationExecute,
    ) {
        maybe_execute_contract(batch, self.dispatcher, execution_msgs.dispatcher);
        maybe_execute_contract(batch, self.timealarms, execution_msgs.timealarms);
        maybe_execute_contract(batch, self.treasury, execution_msgs.treasury);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DexBound<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
}

impl DexBoundContracts {
    fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), platform::error::Error> {
        platform::contract::validate_addr(querier, &self.leaser)
            .and_then(|()| platform::contract::validate_addr(querier, &self.lpp))
            .and_then(|()| platform::contract::validate_addr(querier, &self.oracle))
            .and_then(|()| platform::contract::validate_addr(querier, &self.profit))
    }

    fn migrate(self, batch: &mut Batch, migration_msgs: DexBoundContractsMigration) {
        maybe_migrate_contract(batch, self.leaser, migration_msgs.leaser);
        maybe_migrate_contract(batch, self.lpp, migration_msgs.lpp);
        maybe_migrate_contract(batch, self.oracle, migration_msgs.oracle);
        maybe_migrate_contract(batch, self.profit, migration_msgs.profit);
    }

    fn post_migration_execute(
        self,
        batch: &mut Batch,
        execution_msgs: DexBoundContractsPostMigrationExecute,
    ) {
        maybe_execute_contract(batch, self.leaser, execution_msgs.leaser);
        maybe_execute_contract(batch, self.lpp, execution_msgs.lpp);
        maybe_execute_contract(batch, self.oracle, execution_msgs.oracle);
        maybe_execute_contract(batch, self.profit, execution_msgs.profit);
    }
}
