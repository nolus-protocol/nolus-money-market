use std::collections::HashSet;

use currency::{native::Nls, SymbolOwned};
use finance::percent::Percent;
use lease::api::{ConnectionParams, DownpaymentCoin, InterestPaymentSpec, PositionSpec};
use lpp::{msg::ExecuteMsg, stub::LppRef};
use oracle::stub::OracleRef;
use platform::batch::{Batch, Emit, Emitter};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Deps, StdResult, Storage};

use crate::{
    cmd::Quote,
    error::ContractError,
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
        let config = Config::load(self.deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn customer_leases(&self, customer: Addr) -> StdResult<HashSet<Addr>> {
        Leases::load_by_customer(self.deps.storage, customer)
    }

    pub fn quote(
        &self,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
        max_ltd: Option<Percent>,
    ) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(self.deps.storage)?;

        let lpp = LppRef::try_new(config.lpp_addr, &self.deps.querier)?;

        let oracle = OracleRef::try_from(config.market_price_oracle, &self.deps.querier)?;

        let resp = lpp.execute_lender(
            Quote::new(
                self.deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.lease_position_spec.liability,
                config.lease_interest_rate_margin,
                max_ltd,
            ),
            &self.deps.querier,
        )?;

        Ok(resp)
    }
}

pub(super) fn try_setup_dex(
    storage: &mut dyn Storage,
    params: ConnectionParams,
) -> ContractResult<MessageResponse> {
    Config::setup_dex(storage, params)?;

    Ok(Default::default())
}

pub(super) fn try_configure(
    storage: &mut dyn Storage,
    lease_interest_rate_margin: Percent,
    lease_position_spec: PositionSpec,
    lease_interest_payment: InterestPaymentSpec,
) -> ContractResult<MessageResponse> {
    Config::update(
        storage,
        lease_interest_rate_margin,
        lease_position_spec,
        lease_interest_payment,
    )?;

    Ok(Default::default())
}

pub(super) fn try_migrate_leases(
    storage: &mut dyn Storage,
    new_code_id: u64,
    max_leases: MaxLeases,
) -> ContractResult<MessageResponse> {
    Config::update_lease_code(storage, new_code_id)?;

    let leases = Leases::iter(storage, None);
    migrate::migrate_leases(leases, new_code_id, max_leases)
        .and_then(|result| result.try_add_msgs(|msgs| update_lpp_impl(storage, new_code_id, msgs)))
        .map(|result| {
            MessageResponse::messages_with_events(result.msgs, emit_status(result.next_customer))
        })
}

pub(super) fn try_migrate_leases_cont(
    storage: &mut dyn Storage,
    next_customer: Addr,
    max_leases: MaxLeases,
) -> ContractResult<MessageResponse> {
    let lease_code_id = Config::load(storage)?.lease_code_id;

    let leases = Leases::iter(storage, Some(next_customer));
    migrate::migrate_leases(leases, lease_code_id, max_leases).map(|result| {
        MessageResponse::messages_with_events(result.msgs, emit_status(result.next_customer))
    })
}

pub(super) fn update_lpp(
    storage: &dyn Storage,
    new_code_id: u64,
    mut batch: Batch,
) -> ContractResult<Batch> {
    update_lpp_impl(storage, new_code_id, &mut batch).map(|()| batch)
}

fn update_lpp_impl(
    storage: &dyn Storage,
    new_code_id: u64,
    batch: &mut Batch,
) -> ContractResult<()> {
    let lpp = Config::load(storage)?.lpp_addr;
    let lpp_update_code = ExecuteMsg::NewLeaseCode {
        lease_code_id: new_code_id.into(),
    };
    batch
        .schedule_execute_wasm_no_reply::<_, Nls>(&lpp, lpp_update_code, None)
        .map_err(Into::into)
}

fn emit_status(next_customer: Option<Addr>) -> Emitter {
    let emitter = Emitter::of_type("migrate-leases");
    if let Some(next) = next_customer {
        emitter.emit("contunuation-key", next)
    } else {
        emitter.emit("status", "done")
    }
}
