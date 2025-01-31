use serde::Serialize;

use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{self, Addr, Binary, Storage, WasmMsg};
use versioning::{
    MigrationMessage, PlatformPackageRelease, ProtocolPackageRelease, ProtocolPackageReleaseId,
    SoftwareReleaseId, UpdatablePackage,
};

use crate::{
    error::Error,
    result::Result,
    state::contracts as state_contracts,
    validate::{Validate, ValidateValues},
};

use super::{
    Contracts, ContractsExecute, ContractsMigration, ContractsTemplate, ExecuteSpec, Granularity,
    HigherOrderOption, HigherOrderPlatformContracts, HigherOrderProtocolContracts, HigherOrderType,
    MigrationSpec, PlatformContractAddresses, PlatformExecute, PlatformMigration, Protocol,
    ProtocolContractAddresses, ProtocolExecute, ProtocolMigration, Protocols,
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    to_software_release: SoftwareReleaseId,
    migration_spec: ContractsMigration,
) -> Result<MessageResponse> {
    state_contracts::load_all(storage).and_then(|contracts| {
        contracts.migrate(to_software_release, migration_spec).map(
            |Batches {
                 migration_batch,
                 post_migration_execute_batch,
             }| {
                MessageResponse::messages_only(migration_batch.merge(post_migration_execute_batch))
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
    if let Some(execute_spec) = migration.post_migrate_execute {
        execute_contract(post_migration_execute_batch, address.clone(), execute_spec)?;
    }

    cosmwasm_std::to_json_vec(&MigrationMessage::<Package, _>::new(
        /* TODO Add field once deployed contracts can be queried about their version
            and release information.
        migrate_from,
        */
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
}

impl Contracts {
    fn migrate(
        self,
        software_release: SoftwareReleaseId,
        ContractsMigration { platform, protocol }: ContractsMigration,
    ) -> Result<Batches> {
        let mut migration_batch: Batch = Batch::default();

        let mut post_migration_execute_batch: Batch = Batch::default();

        Self::migrate_platform(
            &mut migration_batch,
            &mut post_migration_execute_batch,
            &software_release,
            self.platform,
            platform,
        )
        .and_then(|()| {
            Self::migrate_protocols(
                &mut migration_batch,
                &mut post_migration_execute_batch,
                software_release,
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
        software_release: &SoftwareReleaseId,
        contracts: PlatformContractAddresses,
        migration_specs: PlatformMigration,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContracts, _, _, _>(
            contracts,
            migration_specs,
            |address, migration_spec| {
                migrate_contract::<PlatformPackageRelease>(
                    migration_batch,
                    post_migration_execute_batch,
                    address,
                    software_release.clone(),
                    migration_spec,
                )
            },
        )
    }

    fn migrate_protocols(
        migration_batch: &mut Batch,
        post_migration_execute_batch: &mut Batch,
        software_release: SoftwareReleaseId,
        protocols: Protocols<Protocol<Addr>>,
        migration_specs: Protocols<ProtocolMigration>,
    ) -> Result<()> {
        Self::try_for_each_protocol_pair(
            protocols,
            migration_specs,
            |contracts, (protocol_release, migration_specs)| {
                Self::try_paired_with_granular::<HigherOrderProtocolContracts, _, _, _>(
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
        contracts: PlatformContractAddresses,
        execute_specs: PlatformExecute,
    ) -> Result<(), Error> {
        Self::try_paired_with_granular::<HigherOrderPlatformContracts, _, _, _>(
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
            Self::try_paired_with_granular::<HigherOrderProtocolContracts, _, _, _>(
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

    fn try_paired_with_granular<HigherOrderType, F, Unit, Err>(
        contracts: HigherOrderType::Of<Addr>,
        paired_with: Granularity<HigherOrderType, HigherOrderOption, Unit>,
        mut f: F,
    ) -> Result<(), Err>
    where
        HigherOrderType: TryForEachPair,
        F: FnMut(Addr, Unit) -> Result<(), Err>,
    {
        match paired_with {
            Granularity::Some { some: paired_with } => HigherOrderType::try_for_each_pair(
                contracts,
                paired_with,
                |address, paired_with| {
                    paired_with.map_or(const { Ok(()) }, |paired_with| f(address, paired_with))
                },
            ),
            Granularity::All(Some(paired_with)) => HigherOrderType::try_for_each_pair(
                contracts,
                paired_with,
                |address, paired_with| f(address, paired_with),
            ),
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

pub(super) trait TryForEach
where
    Self: HigherOrderType,
{
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>;
}

pub(super) trait TryForEachPair
where
    Self: TryForEach + Zip,
{
    fn try_for_each_pair<LeftUnit, RightUnit, F, Err>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
        mut f: F,
    ) -> Result<(), Err>
    where
        F: FnMut(LeftUnit, RightUnit) -> Result<(), Err>,
    {
        Self::try_for_each(Self::zip(left, right), |(left, right)| f(left, right))
    }
}

impl<T> TryForEachPair for T where T: TryForEach + Zip {}

pub(super) trait MapAsRef
where
    Self: HigherOrderType,
{
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T>;
}

pub(super) trait Zip
where
    Self: HigherOrderType,
{
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)>;
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
