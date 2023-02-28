use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::{
    cosmwasm_std::{Addr, Binary, QuerierWrapper, WasmMsg},
    schemars::{self, JsonSchema},
};

use crate::{
    common::type_defs::{Contracts, ContractsMigration, MaybeMigrateContract},
    error::ContractError,
};

pub(crate) mod type_defs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrationSpec<M> {
    pub code_id: u64,
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
}
