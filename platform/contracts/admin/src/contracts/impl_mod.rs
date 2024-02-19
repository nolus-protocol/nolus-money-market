use std::collections::BTreeMap;

use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, Binary, Storage, WasmMsg};
use versioning::ReleaseLabel;

use crate::{
    msg::SudoMsg,
    result::Result,
    state::{contract::Contract as ContractState, contracts as state_contracts},
    validate::{Validate, ValidateValues},
};

use super::{
    protocol::ProtocolTemplate, ContractsGroupedByProtocol, ContractsMigration,
    ContractsPostMigrationExecute, ContractsTemplate, MigrationSpec, Protocol,
};

impl ContractsTemplate<Addr, BTreeMap<String, Protocol>> {
    fn migrate(self, mut migration_msgs: ContractsMigration) -> Result<Batch> {
        let mut batch: Batch = Batch::default();

        self.platform.migrate(&mut batch, migration_msgs.platform);

        self.protocol
            .into_iter()
            .try_for_each(|(name, Protocol { contracts, .. })| {
                migration_msgs
                    .protocol
                    .extract_entry(name)
                    .map(|migration_msgs| contracts.migrate(&mut batch, migration_msgs))
            })
            .map(|()| batch)
    }

    fn post_migration_execute(
        self,
        mut execution_msgs: ContractsPostMigrationExecute,
    ) -> Result<Batch> {
        let mut batch: Batch = Batch::default();

        self.platform
            .post_migration_execute(&mut batch, execution_msgs.platform);

        self.protocol
            .into_iter()
            .try_for_each(|(name, Protocol { contracts, .. })| {
                execution_msgs.protocol.extract_entry(name).map(
                    |execution_msgs: ProtocolTemplate<Option<String>>| {
                        contracts.post_migration_execute(&mut batch, execution_msgs)
                    },
                )
            })
            .map(|()| batch)
    }
}

impl<T, U> Validate for ContractsTemplate<T, BTreeMap<String, U>>
where
    T: Validate,
    U: for<'r> Validate<Context<'r> = T::Context<'r>, Error = T::Error>,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> ::std::result::Result<(), Self::Error> {
        self.platform
            .validate(ctx)
            .and_then(|()| ValidateValues::new(&self.protocol).validate(ctx))
    }
}

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_addr: Addr,
    release: ReleaseLabel,
    migration_spec: ContractsMigration,
    post_migration_execute: ContractsPostMigrationExecute,
) -> Result<MessageResponse> {
    ContractState::AwaitContractsMigrationReply { release }.store(storage)?;

    let contracts_addrs: ContractsGroupedByProtocol = state_contracts::load_all(storage)?;

    contracts_addrs
        .clone()
        .migrate(migration_spec)
        .and_then(|migrate_batch: Batch| {
            contracts_addrs
                .post_migration_execute(post_migration_execute)
                .and_then(|post_migration_execute_batch: Batch| {
                    let mut clear_storage = Batch::default();

                    clear_storage
                        .schedule_execute_wasm_no_reply_no_funds(
                            admin_contract_addr,
                            &SudoMsg::ClearStorage {},
                        )
                        .map(|()| {
                            migrate_batch
                                .merge(clear_storage)
                                .merge(post_migration_execute_batch)
                                .into()
                        })
                        .map_err(Into::into)
                })
        })
}

pub(super) fn maybe_migrate_contract(
    batch: &mut Batch,
    addr: Addr,
    migrate: Option<MigrationSpec>,
) {
    if let Some(migrate) = migrate {
        batch.schedule_execute_reply_on_success(
            WasmMsg::Migrate {
                contract_addr: addr.into_string(),
                new_code_id: migrate.code_id,
                msg: Binary(migrate.migrate_msg.into()),
            },
            0,
        );
    }
}

pub(super) fn maybe_execute_contract(batch: &mut Batch, addr: Addr, execute: Option<String>) {
    if let Some(execute) = execute {
        batch.schedule_execute_no_reply(WasmMsg::Execute {
            contract_addr: addr.into_string(),
            msg: Binary(execute.into()),
            funds: vec![],
        });
    }
}
