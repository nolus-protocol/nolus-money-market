use std::collections::HashSet;

use admin_contract::msg::{ExecuteMsg, MigrationSpec, ProtocolContracts};
use currencies::LeaseGroup;
use currency::CurrencyDTO;
use finance::percent::Percent;
use lease::api::{DownpaymentCoin, MigrateMsg};
use lpp::{msg::ExecuteMsg as LppExecuteMsg, stub::LppRef};
use platform::{
    batch::{Batch, Emit, Emitter},
    contract::Code,
    message::Response as MessageResponse,
};
use reserve::api::ExecuteMsg as ReserveExecuteMsg;
use sdk::cosmwasm_std::{Addr, Deps, Storage};
use versioning::{ProtocolMigrationMessage, ProtocolPackageRelease};

use crate::{
    ContractError,
    cmd::Quote,
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
    lease::Release as LeaseReleaseTrait,
    migrate::{self, MigrationResult},
    msg::{ForceClose, MaxLeases, NewConfig, QuoteResponse},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

pub struct Leaser<'a> {
    deps: Deps<'a>,
}

impl<'a> Leaser<'a> {
    pub fn new(deps: Deps<'a>) -> Self {
        Self { deps }
    }
    pub fn config(&self) -> ContractResult<Config> {
        Config::load(self.deps.storage)
    }

    pub fn customer_leases(&self, customer: Addr) -> ContractResult<HashSet<Addr>> {
        Leases::load_by_customer(self.deps.storage, customer)
    }

    pub fn quote(
        &self,
        downpayment: DownpaymentCoin,
        lease_asset: CurrencyDTO<LeaseGroup>,
        max_ltd: Option<Percent>,
    ) -> ContractResult<QuoteResponse> {
        let config = Config::load(self.deps.storage)?;

        let lpp = LppRef::<LpnCurrency, LpnCurrencies>::try_new(config.lpp, self.deps.querier)?;

        let oracle = OracleRef::try_from_base(config.market_price_oracle, self.deps.querier)?;

        lpp.execute_lender(
            Quote::new(
                self.deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.lease_position_spec.liability,
                config.lease_interest_rate_margin,
                max_ltd,
            ),
            self.deps.querier,
        )
    }
}

pub(super) fn try_configure(
    storage: &mut dyn Storage,
    new_config: NewConfig,
) -> ContractResult<MessageResponse> {
    Config::update(storage, new_config).map(|()| MessageResponse::default())
}

pub(super) fn try_migrate_leases<LeaseRelease, MsgFactory>(
    storage: &mut dyn Storage,
    release_from: LeaseRelease,
    new_lease: Code,
    max_leases: MaxLeases,
    migrate_msg: MsgFactory,
) -> ContractResult<MessageResponse>
where
    LeaseRelease: LeaseReleaseTrait,
    MsgFactory: Fn(ProtocolPackageRelease) -> ProtocolMigrationMessage<MigrateMsg>,
{
    let config = Config::update_lease_code(storage, new_lease)?;

    let customers = Leases::iter(storage, None);

    migrate::migrate_leases(customers, new_lease, release_from, max_leases, migrate_msg)
        .and_then(|result| result.try_add_msgs(|msgs| update_remote_refs(config, msgs)))
        .map(build_response)
}

pub(super) fn try_migrate_leases_cont<LeaseRelease, MsgFactory>(
    storage: &dyn Storage,
    release_from: LeaseRelease,
    next_customer: Addr,
    max_leases: MaxLeases,
    migrate_msg: MsgFactory,
) -> ContractResult<MessageResponse>
where
    LeaseRelease: LeaseReleaseTrait,
    MsgFactory: Fn(ProtocolPackageRelease) -> ProtocolMigrationMessage<MigrateMsg>,
{
    let lease_code = Config::load(storage)?.lease_code;

    let customers = Leases::iter(storage, Some(next_customer));

    migrate::migrate_leases(customers, lease_code, release_from, max_leases, migrate_msg)
        .map(build_response)
}

pub(super) fn try_close_leases<LeaseRelease, MsgFactory>(
    storage: &mut dyn Storage,
    release_from: LeaseRelease,
    new_lease_code: Code,
    max_leases: MaxLeases,
    migrate_msg: MsgFactory,
    force: ForceClose,
) -> ContractResult<MessageResponse>
where
    LeaseRelease: LeaseReleaseTrait,
    MsgFactory: Fn(ProtocolPackageRelease) -> ProtocolMigrationMessage<MigrateMsg>,
{
    match force {
        ForceClose::KillProtocol => try_migrate_leases(
            storage,
            release_from,
            new_lease_code,
            max_leases,
            migrate_msg,
        ),
        ForceClose::No if has_lease(storage) => Err(ContractError::ProtocolStillInUse()),
        ForceClose::No => Ok(MessageResponse::default()),
    }
}

pub(super) fn try_close_protocol<ProtocolsRegistryLoader>(
    storage: &mut dyn Storage,
    protocols_registry: ProtocolsRegistryLoader,
    migration_spec: ProtocolContracts<MigrationSpec>,
) -> ContractResult<MessageResponse>
where
    ProtocolsRegistryLoader: FnOnce(&dyn Storage) -> ContractResult<Addr>,
{
    protocols_registry(storage).and_then(|protocols_registry| {
        let mut batch = Batch::default();
        batch
            .schedule_execute_wasm_no_reply_no_funds(
                protocols_registry,
                &ExecuteMsg::DeregisterProtocol(migration_spec),
            )
            .map_err(ContractError::ProtocolDeregistration)
            .map(|()| batch.into())
    })
}

pub(super) fn try_change_lease_admin(
    storage: &mut dyn Storage,
    new: Addr,
) -> ContractResult<MessageResponse> {
    Config::update_lease_admin(storage, new).map(|_| MessageResponse::default())
}

fn build_response(result: MigrationResult) -> MessageResponse {
    MessageResponse::messages_with_events(result.msgs, emit_status(result.next_customer))
}

fn has_lease(storage: &dyn Storage) -> bool {
    Leases::iter(storage, None).next().is_some()
}

fn update_remote_refs(config: Config, batch: &mut Batch) -> ContractResult<()> {
    let new_lease = config.lease_code;
    {
        let update_msg = LppExecuteMsg::<LpnCurrencies>::NewLeaseCode {
            lease_code: new_lease,
        };

        batch
            .schedule_execute_wasm_no_reply_no_funds(config.lpp, &update_msg)
            .map_err(Into::into)
    }
    .and_then(|()| {
        let update_msg = ReserveExecuteMsg::NewLeaseCode(new_lease);

        batch
            .schedule_execute_wasm_no_reply_no_funds(config.reserve, &update_msg)
            .map_err(Into::into)
    })
}

fn emit_status(next_customer: Option<Addr>) -> Emitter {
    let emitter = Emitter::of_type("migrate-leases");
    if let Some(next) = next_customer {
        emitter.emit("contunuation-key", next)
    } else {
        emitter.emit("status", "done")
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use admin_contract::msg::{MigrationSpec, ProtocolContracts};
    use json_value::JsonValue;
    use lease::api::MigrateMsg;
    use platform::{contract::Code, response};
    use sdk::cosmwasm_std::{Addr, Storage, testing::MockStorage};
    use versioning::{
        ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    };

    use crate::{
        ContractError,
        lease::{Release, test::FixedRelease},
        msg::{ForceClose, MaxLeases},
        result::ContractResult,
        state::leases::Leases,
        tests,
    };

    const MAX_LEASES: MaxLeases = 100_000;
    const LEASE_VOID_CODE: Code = Code::unchecked(12);

    #[test]
    fn close_empty_protocol() {
        let mut store = MockStorage::default();
        tests::config().store(&mut store).unwrap();
        let resp = super::try_close_leases(
            &mut store,
            dummy_release(),
            LEASE_VOID_CODE,
            MAX_LEASES,
            migrate_msg,
            ForceClose::No,
        )
        .and_then(|leases_resp| {
            super::try_close_protocol(&mut store, protocols_registry, dummy_spec())
                .map(|protocol_resp| leases_resp.merge_with(protocol_resp))
        })
        .unwrap();
        let cw_resp = response::response_only_messages(resp);
        let delete_protocol = 1;
        assert_eq!(delete_protocol, cw_resp.messages.len());
    }

    #[test]
    fn close_non_empty_protocol() {
        let mut store = MockStorage::default();
        tests::config().store(&mut store).unwrap();
        let customer = Addr::unchecked("CustomerA");
        let lease = Addr::unchecked("Lease1");
        Leases::cache_open_req(&mut store, &customer).expect("cache the customer should succeed");
        Leases::save(&mut store, lease).expect("save a new lease should succeed");
        assert_eq!(
            Err(ContractError::ProtocolStillInUse()),
            super::try_close_leases(
                &mut store,
                dummy_release(),
                LEASE_VOID_CODE,
                MAX_LEASES,
                migrate_msg,
                ForceClose::No
            )
        );

        let resp = super::try_close_leases(
            &mut store,
            dummy_release(),
            LEASE_VOID_CODE,
            MAX_LEASES,
            migrate_msg,
            ForceClose::KillProtocol,
        )
        .unwrap();
        let cw_resp = response::response_only_messages(resp);
        let update_lpp_update_reserve_migrate_legacy_leases = 1 + 1 + 1;
        assert_eq!(
            update_lpp_update_reserve_migrate_legacy_leases,
            cw_resp.messages.len()
        );
    }

    fn dummy_spec() -> ProtocolContracts<MigrationSpec> {
        let migration_spec = MigrationSpec {
            code_id: 23u64.into(),
            migrate_message: JsonValue::Object(vec![]),
            post_migrate_execute: None,
        };
        ProtocolContracts {
            leaser: migration_spec.clone(),
            lpp: migration_spec.clone(),
            oracle: migration_spec.clone(),
            profit: migration_spec.clone(),
            reserve: migration_spec,
        }
    }

    fn dummy_release() -> impl Release {
        FixedRelease::with(ProtocolPackageRelease::current("moduleX", "0.1.2", 1))
    }

    fn migrate_msg(migrate_from: ProtocolPackageRelease) -> ProtocolMigrationMessage<MigrateMsg> {
        ProtocolMigrationMessage {
            migrate_from,
            to_release: ProtocolPackageReleaseId::new(
                ReleaseId::new_test("v0.5.4"),
                ReleaseId::new_test("v0.2.1"),
            ),
            message: MigrateMsg {},
        }
    }

    fn protocols_registry(_storage: &dyn Storage) -> ContractResult<Addr> {
        Ok(Addr::unchecked("Registry"))
    }
}
