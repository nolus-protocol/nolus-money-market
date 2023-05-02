use currency::native::Nls;
use finance::duration::Duration;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, Env, QuerierWrapper, Storage, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{msg::ConfigResponse, result::ContractResult, state::config::Config, ContractError};

pub struct Profit {}

impl Profit {
    pub(crate) fn try_config(storage: &mut dyn Storage, cadence_hours: u16) -> ContractResult<()> {
        Config::update(storage, cadence_hours)
    }

    pub(crate) fn transfer(
        deps: Deps<'_>,
        env: &Env,
        timealarms: Addr,
    ) -> ContractResult<MessageResponse> {
        let config = Config::load(deps.storage)?;

        let balance = deps.querier.query_all_balances(&env.contract.address)?;

        if balance.is_empty() {
            return Err(ContractError::EmptyBalance {});
        }

        Self::setup_alarm(
            timealarms,
            &deps.querier,
            env.block.time,
            Duration::from_hours(config.cadence_hours),
        )
        .and_then(|time_alarm| add_transfers(time_alarm, env, &deps.querier, config))
        // TODO add in_stable(wasm-tr-profit.profit-amount) The amount transferred in stable.
        //.emit_coin("profit-amount", balance))
    }

    pub fn query_config(storage: &dyn Storage) -> ContractResult<ConfigResponse> {
        Config::load(storage).map(|config| ConfigResponse {
            cadence_hours: config.cadence_hours,
        })
    }

    pub(crate) fn setup_alarm(
        timealarms: Addr,
        querier: &QuerierWrapper<'_>,
        current_time: Timestamp,
        cadence: Duration,
    ) -> ContractResult<Batch> {
        TimeAlarmsRef::new(timealarms, querier)
            .and_then(|timealarms| timealarms.setup_alarm(current_time + cadence))
            .map_err(Into::into)
    }
}

fn add_transfers(
    messages: Batch,
    env: &Env,
    querier: &QuerierWrapper<'_>,
    config: Config,
) -> ContractResult<MessageResponse> {
    let mut bank = bank::account(&env.contract.address, querier);
    bank.balance::<Nls>()
        .map(|balance| {
            bank.send(balance, &config.treasury);
            Emitter::of_type("tr-profit")
                .emit_tx_info(env)
                .emit_coin("profit-amount", balance)
        })
        .map(|emitter| {
            MessageResponse::messages_with_events(
                Into::<Batch>::into(bank).merge(messages),
                emitter,
            )
        })
        .map_err(Into::into)
}
