use std::collections::HashSet;

use admin_contract::msg::{ExecuteMsg, MigrationSpec, ProtocolContracts};
use currency::SymbolOwned;
use finance::{duration::Duration, percent::Percent};
use lease::api::{open::PositionSpecDTO, DownpaymentCoin, MigrateMsg};
use lpp::{msg::ExecuteMsg as LppExecuteMsg, stub::LppRef};
use platform::{
    batch::{Batch, Emit, Emitter},
    contract::Code,
    message::Response as MessageResponse,
};
use reserve::api::ExecuteMsg as ReserveExecuteMsg;
use sdk::cosmwasm_std::{Addr, Deps, Storage};

use crate::{
    cmd::Quote,
    finance::LpnCurrencies,
    migrate,
    msg::{ConfigResponse, MaxLeases, QuoteResponse},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};
use crate::{
    finance::{LpnCurrency, OracleRef},
    msg::ForceClose,
    ContractError,
};

pub struct Leaser<'a> {
    deps: Deps<'a>,
}

impl<'a> Leaser<'a> {
    pub fn new(deps: Deps<'a>) -> Self {
        Self { deps }
    }
    pub fn config(&self) -> ContractResult<ConfigResponse> {
        Config::load(self.deps.storage).map(|config| ConfigResponse { config })
    }

    pub fn customer_leases(&self, customer: Addr) -> ContractResult<HashSet<Addr>> {
        Leases::load_by_customer(self.deps.storage, customer)
    }

    pub fn quote(
        &self,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
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
    lease_interest_rate_margin: Percent,
    lease_position_spec: PositionSpecDTO,
    lease_due_period: Duration,
) -> ContractResult<MessageResponse> {
    Config::update(
        storage,
        lease_interest_rate_margin,
        lease_position_spec,
        lease_due_period,
    )
    .map(|()| MessageResponse::default())
}

pub(super) fn try_migrate_leases<MsgFactory>(
    storage: &mut dyn Storage,
    new_lease: Code,
    max_leases: MaxLeases,
    migrate_msg: MsgFactory,
) -> ContractResult<MessageResponse>
where
    MsgFactory: Fn(Addr) -> MigrateMsg,
{
    Config::update_lease_code(storage, new_lease)?;

    let leases = Leases::iter(storage, None);
    migrate::migrate_leases(leases, new_lease, max_leases, migrate_msg)
        .and_then(|result| result.try_add_msgs(|msgs| update_remote_refs(storage, new_lease, msgs)))
        .map(|result| {
            MessageResponse::messages_with_events(result.msgs, emit_status(result.next_customer))
        })
}

pub(super) fn try_migrate_leases_cont<MsgFactory>(
    storage: &mut dyn Storage,
    next_customer: Addr,
    max_leases: MaxLeases,
    migrate_msg: MsgFactory,
) -> ContractResult<MessageResponse>
where
    MsgFactory: Fn(Addr) -> MigrateMsg,
{
    let lease_code = Config::load(storage)?.lease_code;

    let leases = Leases::iter(storage, Some(next_customer));
    migrate::migrate_leases(leases, lease_code, max_leases, migrate_msg).map(|result| {
        MessageResponse::messages_with_events(result.msgs, emit_status(result.next_customer))
    })
}

pub(super) fn try_close_protocol<ProtocolsRegistryLoader>(
    storage: &dyn Storage,
    protocols_registry: ProtocolsRegistryLoader,
    migration_spec: ProtocolContracts<MigrationSpec>,
    force: ForceClose,
) -> ContractResult<MessageResponse>
where
    ProtocolsRegistryLoader: FnOnce(&dyn Storage) -> ContractResult<Addr>,
{
    if force == ForceClose::No && has_lease(storage) {
        Err(ContractError::ProtocolStillInUse())
    } else {
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
}

fn has_lease(storage: &dyn Storage) -> bool {
    Leases::iter(storage, None).next().is_some()
}

fn update_remote_refs(
    storage: &dyn Storage,
    new_lease: Code,
    batch: &mut Batch,
) -> ContractResult<()> {
    let cfg = Config::load(storage)?;
    {
        let update_msg = LppExecuteMsg::<LpnCurrencies>::NewLeaseCode {
            lease_code: new_lease,
        };
        batch
            .schedule_execute_wasm_no_reply_no_funds(cfg.lpp, &update_msg)
            .map_err(Into::into)
    }
    .and_then(|()| {
        let update_msg = ReserveExecuteMsg::NewLeaseCode(new_lease);
        batch
            .schedule_execute_wasm_no_reply_no_funds(cfg.reserve, &update_msg)
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

#[cfg(test)]
mod test {
    use admin_contract::msg::{MigrationSpec, ProtocolContracts};
    use cosmwasm_std::Addr;
    use sdk::cosmwasm_std::testing::MockStorage;

    use crate::{msg::ForceClose, state::leases::Leases, ContractError};

    #[test]
    fn close_empty_protocol() {
        let store = MockStorage::default();
        assert!(super::try_close_protocol(
            &store,
            |_| Ok(Addr::unchecked("Registry")),
            dummy_spec(),
            ForceClose::No
        )
        .is_ok());
    }

    #[test]
    fn close_non_empty_protocol() {
        let mut store = MockStorage::default();
        let customer = Addr::unchecked("CustomerA");
        let lease = Addr::unchecked("Lease1");
        Leases::cache_open_req(&mut store, &customer).expect("cache the customer should succeed");
        Leases::save(&mut store, lease).expect("save a new lease should succeed");
        assert_eq!(
            Err(ContractError::ProtocolStillInUse()),
            super::try_close_protocol(
                &store,
                |_| Ok(Addr::unchecked("Registry")),
                dummy_spec(),
                ForceClose::No
            )
        );

        assert!(super::try_close_protocol(
            &store,
            |_| Ok(Addr::unchecked("Registry")),
            dummy_spec(),
            ForceClose::KillProtocol
        )
        .is_ok());
    }

    fn dummy_spec() -> ProtocolContracts<MigrationSpec> {
        let migration_spec = MigrationSpec {
            code_id: 23u64.into(),
            migrate_msg: "{}".into(),
            post_migrate_execute_msg: None,
        };
        ProtocolContracts {
            leaser: migration_spec.clone(),
            lpp: migration_spec.clone(),
            oracle: migration_spec.clone(),
            profit: migration_spec.clone(),
            reserve: migration_spec,
        }
    }
}
