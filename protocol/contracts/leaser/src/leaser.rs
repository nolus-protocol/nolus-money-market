use std::collections::HashSet;

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

use crate::finance::{LpnCurrency, OracleRef};
use crate::{
    cmd::Quote,
    finance::LpnCurrencies,
    migrate,
    msg::{ConfigResponse, MaxLeases, QuoteResponse},
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
