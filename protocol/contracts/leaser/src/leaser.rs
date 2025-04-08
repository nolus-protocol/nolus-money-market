use std::collections::HashSet;

use admin_contract::msg::{ExecuteMsg, MigrationSpec, ProtocolContracts};
use currencies::LeaseGroup;
use currency::CurrencyDTO;
use finance::percent::{Percent, Percent100};
use lease::api::{DownpaymentCoin, MigrateMsg};
use lpp::{
    msg::ExecuteMsg as LppExecuteMsg,
    stub::{
        deposit::{Depositer, WithDepositer},
        LppRef,
    },
};
use platform::{
    batch::{Batch, Emit, Emitter},
    contract::Code,
    message::Response as MessageResponse,
};
use reserve::api::ExecuteMsg as ReserveExecuteMsg;
use sdk::cosmwasm_std::{Addr, Deps, QuerierWrapper, Storage};
use versioning::{ProtocolMigrationMessage, ProtocolPackageRelease};

use crate::{
    cmd::Quote,
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
    lease::Release as LeaseReleaseTrait,
    migrate::{self, MigrationResult},
    msg::{MaxLeases, NewConfig, QuoteResponse},
    result::ContractResult,
    state::{config::Config, leases::Leases},
    ContractError,
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

        let lpp = LppRef::<LpnCurrency>::try_new(config.lpp, self.deps.querier)
            .map_err(ContractError::LppStubCreation)?;

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

pub(crate) fn try_close_deposits(
    storage: &mut dyn Storage,
    querier: QuerierWrapper<'_>,
) -> ContractResult<MessageResponse> {
    struct Cmd {}
    impl WithDepositer<LpnCurrency> for Cmd {
        type Output = Batch;

        type Error = ContractError;

        fn exec<Lpp>(self, mut lpp: Lpp) -> Result<Self::Output, Self::Error>
        where
            Lpp: Depositer<LpnCurrency>,
        {
            lpp.close_all()
                .map_err(ContractError::CloseAllDeposits)
                .map(|()| lpp.into())
        }
    }
    Config::load(storage)
        .and_then(|config| {
            LppRef::<LpnCurrency>::try_new(config.lpp, querier)
                .map_err(ContractError::LppStubCreation)
        })
        .and_then(|lpp_ref| lpp_ref.execute_depositer(Cmd {}).map(Into::into))
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
    use platform::response;
    use sdk::cosmwasm_std::{testing::MockStorage, Addr, Storage};

    use crate::{result::ContractResult, tests};

    #[test]
    fn close_empty_protocol() {
        let mut store = MockStorage::default();
        tests::config().store(&mut store).unwrap();
        let resp = super::try_close_protocol(&mut store, protocols_registry, dummy_spec()).unwrap();
        let cw_resp = response::response_only_messages(resp);
        let delete_protocol = 1;
        assert_eq!(delete_protocol, cw_resp.messages.len());
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

    fn protocols_registry(_storage: &dyn Storage) -> ContractResult<Addr> {
        Ok(Addr::unchecked("Registry"))
    }
}
