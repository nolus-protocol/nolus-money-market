use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, Binary, Storage, WasmMsg};
use versioning::ReleaseId;

use crate::{
    error::Error,
    msg::ExecuteMsg,
    result::Result,
    state::{contract::Contract as ContractState, contracts as state_contracts},
    validate::{Validate, ValidateValues},
};

use super::{
    higher_order_type::TryForEachPair, Contracts, ContractsExecute, ContractsMigration,
    ContractsTemplate, Granularity, HigherOrderOption, HigherOrderPlatformContracts,
    HigherOrderProtocolContracts, HigherOrderType, MigrationSpec, PlatformContractAddresses,
    PlatformExecute, PlatformMigration, Protocol, ProtocolContractAddresses, ProtocolExecute,
    ProtocolMigration, Protocols,
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_addr: Addr,
    release: ReleaseId,
    migration_spec: ContractsMigration,
) -> Result<MessageResponse> {
    ContractState::AwaitContractsMigrationReply { release }.store(storage)?;

    state_contracts::load_all(storage).and_then(|contracts| {
        contracts.migrate(migration_spec).and_then(
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
    })
}

pub(crate) fn execute(
    storage: &mut dyn Storage,
    execute_messages: ContractsExecute,
) -> Result<MessageResponse> {
    state_contracts::load_all(storage)
        .and_then(|contracts| contracts.execute(execute_messages))
        .map(MessageResponse::messages_only)
}

pub(super) fn migrate_contract(
    migration_batch: &mut Batch,
    post_migration_execute_batch: &mut Batch,
    address: Addr,
    migrate: MigrationSpec,
) -> Result<()> {
    migrate
        .post_migrate_execute_msg
        .map_or(const { Ok(()) }, |post_migrate_execute_msg| {
            execute_contract(
                post_migration_execute_batch,
                address.clone(),
                post_migrate_execute_msg,
            )
        })
        .map(|()| {
            migration_batch.schedule_execute_reply_on_success(
                WasmMsg::Migrate {
                    contract_addr: address.into_string(),
                    new_code_id: migrate.code_id.u64(),
                    msg: Binary::new(migrate.migrate_msg.into_bytes()),
                },
                0,
            )
        })
}

impl Contracts {
    fn migrate(
        self,
        ContractsMigration { platform, protocol }: ContractsMigration,
    ) -> Result<Batches> {
        let mut migration_batch: Batch = Batch::default();

        let mut post_migration_execute_batch: Batch = Batch::default();

        Self::migrate_platform(
            &mut migration_batch,
            &mut post_migration_execute_batch,
            self.platform,
            platform,
        )
        .and_then(|()| {
            Self::migrate_protocols(
                &mut migration_batch,
                &mut post_migration_execute_batch,
                self.protocol,
                protocol,
            )
            .map(|()| Batches {
                migration_batch,
                post_migration_execute_batch,
            })
        })
    }

    fn migrate_platform(
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        contracts: PlatformContractAddresses,
        migration_specs: PlatformMigration,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContracts, _, _, _, _>(
            contracts,
            migration_specs,
            |address, migration_spec| {
                migrate_contract(
                    migration_batch,
                    post_migration_execute_batch,
                    address,
                    migration_spec,
                )
            },
        )
    }

    fn migrate_protocols(
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        protocols: Protocols<Protocol<Addr>>,
        migration_specs: Protocols<ProtocolMigration>,
    ) -> Result<()> {
        Self::try_for_each_protocol_pair(
            protocols,
            migration_specs,
            |contracts, migration_specs| {
                Self::try_paired_with_granular::<HigherOrderProtocolContracts, _, _, _, _>(
                    contracts,
                    migration_specs,
                    |address, migrate_spec| {
                        migrate_contract(
                            migration_batch,
                            post_migration_execute_batch,
                            address,
                            migrate_spec,
                        )
                    },
                )
            },
        )
    }

    fn execute(self, ContractsExecute { platform, protocol }: ContractsExecute) -> Result<Batch> {
        let mut batch: Batch = Batch::default();

        Self::execute_platform(&mut batch, self.platform, platform).and_then(|()| {
            Self::execute_protocols(&mut batch, self.protocol, protocol).map(|()| batch)
        })
    }

    fn execute_platform(
        batch: &mut Batch,
        contracts: PlatformContractAddresses,
        execute_specs: PlatformExecute,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContracts, _, _, _, _>(
            contracts,
            execute_specs,
            |address, execute_spec| execute_contract(batch, address, execute_spec),
        )
    }

    fn execute_protocols(
        batch: &mut Batch,
        contracts: Protocols<Protocol<Addr>>,
        execute_specs: Protocols<ProtocolExecute>,
    ) -> Result<()> {
        Self::try_for_each_protocol_pair(contracts, execute_specs, |contracts, execute_specs| {
            Self::try_paired_with_granular::<HigherOrderProtocolContracts, _, _, _, _>(
                contracts,
                execute_specs,
                |address, execute_spec| execute_contract(batch, address, execute_spec),
            )
        })
    }

    fn try_for_each_protocol_pair<T, F>(
        protocols: Protocols<Protocol<Addr>>,
        mut paired_with: Protocols<T>,
        mut f: F,
    ) -> Result<()>
    where
        F: FnMut(ProtocolContractAddresses, T) -> Result<()>,
    {
        protocols
            .into_iter()
            .try_for_each(|(name, Protocol { contracts, .. })| {
                paired_with
                    .remove(&name)
                    .ok_or_else(|| Error::MissingProtocol(name))
                    .and_then(|protocol| f(contracts, protocol))
            })
            .and_then(|()| {
                paired_with
                    .pop_first()
                    .map_or(const { Ok(()) }, |(name, _)| {
                        Err(Error::UnknownProtocol(name))
                    })
            })
    }

    fn try_paired_with_granular<HigherOrderType, Unit, GranularUnit, F, Err>(
        instance: HigherOrderType::Of<Unit>,
        paired_with: Granularity<HigherOrderType, HigherOrderOption, GranularUnit>,
        mut f: F,
    ) -> Result<(), Err>
    where
        HigherOrderType: TryForEachPair,
        F: FnMut(Unit, GranularUnit) -> Result<(), Err>,
    {
        match paired_with {
            Granularity::Some { some: paired_with } => {
                HigherOrderType::try_for_each_pair(instance, paired_with, |unit, paired_with| {
                    paired_with.map_or(const { Ok(()) }, |paired_with| f(unit, paired_with))
                })
            }
            Granularity::All(Some(paired_with)) => {
                HigherOrderType::try_for_each_pair(instance, paired_with, |unit, paired_with| {
                    f(unit, paired_with)
                })
            }
            Granularity::All(None) => const { Ok(()) },
        }
    }
}

impl<Platform, Protocol, Unit> Validate for ContractsTemplate<Platform, Protocol, Unit>
where
    Platform: HigherOrderType,
    Platform::Of<Unit>: Validate,
    Protocol: HigherOrderType,
    Protocol::Of<Unit>: for<'r> Validate<
        Context<'r> = <Platform::Of<Unit> as Validate>::Context<'r>,
        Error = <Platform::Of<Unit> as Validate>::Error,
    >,
{
    type Context<'r> = <Platform::Of<Unit> as Validate>::Context<'r>;

    type Error = <Platform::Of<Unit> as Validate>::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.platform
            .validate(ctx)
            .and_then(|()| ValidateValues::new(&self.protocol).validate(ctx))
    }
}

fn execute_contract(batch: &mut Batch, address: Addr, message: String) -> Result<()> {
    batch.schedule_execute_no_reply(WasmMsg::Execute {
        contract_addr: address.into_string(),
        msg: Binary::new(message.into_bytes()),
        funds: vec![],
    });

    Ok(())
}

struct Batches {
    migration_batch: Batch,
    post_migration_execute_batch: Batch,
}
