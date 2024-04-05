use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, Binary, Storage, WasmMsg};
use versioning::ReleaseLabel;

use crate::{
    error::Error,
    msg::ExecuteMsg,
    result::Result,
    state::{contract::Contract as ContractState, contracts as state_contracts},
    validate::{Validate, ValidateValues},
};

use super::{Contracts, ContractsMigration, ContractsTemplate, Identity, MigrationSpec, Protocol};

impl Contracts {
    fn migrate(self, mut migration_msgs: ContractsMigration) -> Result<Batches> {
        let mut migration_batch: Batch = Batch::default();

        let mut post_migration_execute_batch: Batch = Batch::default();

        if let Some(platform) = migration_msgs.platform {
            self.platform.migrate(
                &mut migration_batch,
                &mut post_migration_execute_batch,
                platform,
            );
        }

        self.protocol
            .into_iter()
            .try_for_each(|(name, Protocol { contracts, .. })| {
                migration_msgs
                    .protocol
                    .remove(&name)
                    .ok_or_else(|| Error::MissingProtocol(name))
                    .map(|protocol| {
                        protocol.map_or((), |protocol| {
                            contracts.migrate(
                                &mut migration_batch,
                                &mut post_migration_execute_batch,
                                protocol,
                            )
                        })
                    })
            })
            .map(|()| Batches {
                migration_batch,
                post_migration_execute_batch,
            })
    }
}

impl<T, U> Validate for ContractsTemplate<Identity, T, U>
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
) -> Result<MessageResponse> {
    ContractState::AwaitContractsMigrationReply { release }.store(storage)?;

    state_contracts::load_all(storage)?
        .migrate(migration_spec)
        .and_then(
            |Batches {
                 mut migration_batch,
                 post_migration_execute_batch,
             }| {
                migration_batch
                    .schedule_execute_wasm_no_reply_no_funds(
                        admin_contract_addr,
                        &ExecuteMsg::EndOfMigration {},
                    )
                    .map(|()| {
                        MessageResponse::messages_only(
                            migration_batch.merge(post_migration_execute_batch),
                        )
                    })
                    .map_err(Into::into)
            },
        )
}

pub(super) fn migrate_contract(
    migration_batch: &mut Batch,
    post_migration_execute_batch: &mut Batch,
    address: Addr,
    migrate: MigrationSpec,
) {
    if let Some(post_migrate_execute_msg) = migrate.post_migrate_execute_msg {
        post_migration_execute_batch.schedule_execute_no_reply(WasmMsg::Execute {
            contract_addr: address.clone().into_string(),
            msg: Binary(post_migrate_execute_msg.into_bytes()),
            funds: vec![],
        });
    }

    migration_batch.schedule_execute_reply_on_success(
        WasmMsg::Migrate {
            contract_addr: address.into_string(),
            new_code_id: migrate.code_id.u64(),
            msg: Binary(migrate.migrate_msg.into_bytes()),
        },
        0,
    );
}

struct Batches {
    migration_batch: Batch,
    post_migration_execute_batch: Batch,
}
