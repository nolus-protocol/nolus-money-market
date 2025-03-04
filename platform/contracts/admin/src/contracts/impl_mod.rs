use serde::Serialize;

use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{self, Addr, Binary, Storage, WasmMsg};
use versioning::{
    MigrationMessage, PlatformPackageRelease, ProtocolPackageRelease, ProtocolPackageReleaseId,
    ReleaseId, UpdatablePackage,
};

use crate::{
    error::Error,
    result::Result,
    state::contracts as state_contracts,
    validate::{Validate, ValidateValues},
};

use super::{
    Contracts, ContractsExecute, ContractsMigration, ContractsTemplate, ExecuteSpec, Granularity,
    HigherOrderOption, HigherOrderPlatformContracts, HigherOrderPlatformContractsWithoutAdmin,
    HigherOrderProtocolContracts, HigherOrderType, MigrationSpec, PlatformContractAddresses,
    PlatformContractAddressesWithoutAdmin, PlatformExecute, PlatformMigration, Protocol,
    ProtocolContractAddresses, ProtocolExecute, ProtocolMigration, Protocols,
    higher_order_type::TryForEachPair,
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract: Addr,
    to_software_release: ReleaseId,
    migration_spec: ContractsMigration,
) -> Result<MessageResponse> {
    state_contracts::load_all(storage).and_then(|contracts| {
        contracts
            .migrate(admin_contract, to_software_release, migration_spec)
            .map(|batches| MessageResponse::messages_only(batches.merge()))
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

pub(super) fn migrate_contract<Package>(
    migration_batch: &mut Batch,
    post_migration_execute_batch: &mut Batch,
    address: Addr,
    /* TODO Add field once deployed contracts can be queried about their version
        and release information.
    migrate_from: Package,
    */
    to_release: Package::ReleaseId,
    migration: MigrationSpec,
) -> Result<()>
where
    Package: UpdatablePackage,
    Package::ReleaseId: Serialize,
{
    migration
        .post_migrate_execute
        .map_or(const { Ok(()) }, |post_migrate_execute_msg| {
            execute_contract(
                post_migration_execute_batch,
                address.clone(),
                post_migrate_execute_msg,
            )
        })
        .and_then(|()| {
            cosmwasm_std::to_json_vec(&MigrationMessage::<Package, _>::new(
                to_release,
                migration.migrate_message,
            ))
            .map(|message| {
                migration_batch.schedule_execute_no_reply(WasmMsg::Migrate {
                    contract_addr: address.into_string(),
                    new_code_id: migration.code_id.u64(),
                    msg: Binary::new(message),
                })
            })
            .map_err(Into::into)
        })
}

impl Contracts {
    fn migrate(
        self,
        admin_contract: Addr,
        to_software_release: ReleaseId,
        ContractsMigration { platform, protocol }: ContractsMigration,
    ) -> Result<Batches> {
        let mut migration_batch: Batch = Batch::default();

        let mut post_migration_execute_batch: Batch = Batch::default();

        Self::migrate_platform(
            &mut migration_batch,
            &mut post_migration_execute_batch,
            &to_software_release,
            self.platform.with_admin(admin_contract),
            platform,
        )
        .and_then(|()| {
            Self::migrate_protocols(
                &mut migration_batch,
                &mut post_migration_execute_batch,
                to_software_release,
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
        to_software_release: &ReleaseId,
        contracts: PlatformContractAddresses,
        migration_specs: PlatformMigration,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContracts, _, _, _, _>(
            contracts,
            migration_specs,
            |address, migration_spec| {
                migrate_contract::<PlatformPackageRelease>(
                    migration_batch,
                    post_migration_execute_batch,
                    address,
                    to_software_release.clone(),
                    migration_spec,
                )
            },
        )
    }

    fn migrate_protocols(
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        software_release: ReleaseId,
        protocols: Protocols<Protocol<Addr>>,
        migration_specs: Protocols<ProtocolMigration>,
    ) -> Result<()> {
        Self::try_for_each_protocol_pair(
            protocols,
            migration_specs,
            |contracts, (protocol_release, migration_specs)| {
                Self::try_paired_with_granular::<HigherOrderProtocolContracts, _, _, _, _>(
                    contracts,
                    migration_specs,
                    |address, migrate_spec| {
                        migrate_contract::<ProtocolPackageRelease>(
                            migration_batch,
                            post_migration_execute_batch,
                            address,
                            ProtocolPackageReleaseId::new(
                                software_release.clone(),
                                protocol_release.clone(),
                            ),
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
        contracts: PlatformContractAddressesWithoutAdmin,
        execute_specs: PlatformExecute,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContractsWithoutAdmin, _, _, _, _>(
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

fn execute_contract(
    batch: &mut Batch,
    address: Addr,
    ExecuteSpec { message }: ExecuteSpec,
) -> Result<()> {
    cosmwasm_std::to_json_vec(&message)
        .map(|message| {
            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: address.into_string(),
                msg: Binary::new(message),
                funds: vec![],
            })
        })
        .map_err(Into::into)
}

struct Batches {
    migration_batch: Batch,
    post_migration_execute_batch: Batch,
}

impl Batches {
    fn merge(self) -> Batch {
        self.migration_batch
            .merge(self.post_migration_execute_batch)
    }
}
