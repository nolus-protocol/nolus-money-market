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
    error::ContractError,
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
#[serde(rename_all = "snake_case")]
pub struct ContractsTemplate<T> {
    pub dispatcher: T,
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
    pub timealarms: T,
    pub treasury: T,
}

impl Contracts {
    pub(crate) fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), ContractError> {
        platform::contract::validate_addr(querier, &self.dispatcher)?;
        platform::contract::validate_addr(querier, &self.leaser)?;
        platform::contract::validate_addr(querier, &self.lpp)?;
        platform::contract::validate_addr(querier, &self.oracle)?;
        platform::contract::validate_addr(querier, &self.profit)?;
        platform::contract::validate_addr(querier, &self.timealarms)?;
        platform::contract::validate_addr(querier, &self.treasury).map_err(Into::into)
    }

    pub(crate) fn migrate(self, migration_msgs: ContractsMigration) -> Batch {
        let mut batch: Batch = Batch::default();

        maybe_migrate_contract(&mut batch, self.dispatcher, migration_msgs.dispatcher);
        maybe_migrate_contract(&mut batch, self.leaser, migration_msgs.leaser);
        maybe_migrate_contract(&mut batch, self.lpp, migration_msgs.lpp);
        maybe_migrate_contract(&mut batch, self.oracle, migration_msgs.oracle);
        maybe_migrate_contract(&mut batch, self.profit, migration_msgs.profit);
        maybe_migrate_contract(&mut batch, self.timealarms, migration_msgs.timealarms);
        maybe_migrate_contract(&mut batch, self.treasury, migration_msgs.treasury);

        batch
    }

    pub(crate) fn post_migration_execute(
        self,
        execution_msgs: ContractsPostMigrationExecute,
    ) -> Batch {
        let mut batch: Batch = Batch::default();

        maybe_execute_contract(&mut batch, self.dispatcher, execution_msgs.dispatcher);
        maybe_execute_contract(&mut batch, self.leaser, execution_msgs.leaser);
        maybe_execute_contract(&mut batch, self.lpp, execution_msgs.lpp);
        maybe_execute_contract(&mut batch, self.oracle, execution_msgs.oracle);
        maybe_execute_contract(&mut batch, self.profit, execution_msgs.profit);
        maybe_execute_contract(&mut batch, self.timealarms, execution_msgs.timealarms);
        maybe_execute_contract(&mut batch, self.treasury, execution_msgs.treasury);

        batch
    }
}
