use currencies::PaymentGroup;
use cw_time::IntoInstant;
use finance::{duration::Duration, instant::Instant};
use platform::{
    bank,
    batch::{Batch, Emit as _, Emitter},
    message::Response as PlatformResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{result::ContractResult, state::Config};

const PROFIT_EVENT_TYPE: &str = "tr-profit";

pub(crate) fn on_time_alarm(
    config: &Config,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<PlatformResponse> {
    setup_alarm(config, env.block.time.into_instant()).and_then(|alarm: Batch| {
        sweep(config.settlement(), &env.contract.address, querier, env)
            .map(|sweep: PlatformResponse| sweep.merge_with(PlatformResponse::messages_only(alarm)))
    })
}

pub(crate) fn setup_alarm(config: &Config, now: Instant) -> ContractResult<Batch> {
    config
        .time_alarms()
        .setup_alarm(now + Duration::from_hours(config.cadence_hours()))
        .map_err(Into::into)
}

fn sweep(
    settlement: &Addr,
    profit: &Addr,
    querier: QuerierWrapper<'_>,
    env: &Env,
) -> ContractResult<PlatformResponse> {
    bank::bank_send_all::<PaymentGroup>(profit, settlement.clone(), querier)
        .map(|batch: Batch| {
            PlatformResponse::messages_with_event(
                batch,
                Emitter::of_type(PROFIT_EVENT_TYPE).emit_tx_info(env),
            )
        })
        .map_err(Into::into)
}
