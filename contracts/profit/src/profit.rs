use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::BankAccount,
    batch::{Batch, Emit as _, Emitter},
    error::Error as PlatformError,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper, Storage};

use crate::{
    msg::ConfigResponse,
    result::ContractResult,
    state::{
        config::Config,
        contract_state::{State, STATE},
    },
};

pub struct Profit;

impl Profit {
    pub(crate) fn transfer_nls<B>(
        mut account: B,
        env: &Env,
        treasury_addr: &Addr,
    ) -> Result<(Batch, Emitter), PlatformError>
    where
        B: BankAccount,
    {
        let balance_nls: Coin<Nls> = account.balance()?;

        account.send(balance_nls, treasury_addr);

        Ok((
            account.into(),
            Emitter::of_type("tr-profit")
                .emit_tx_info(env)
                .emit_coin("profit-amount", balance_nls),
        ))
    }

    pub fn query_config(storage: &dyn Storage) -> ContractResult<ConfigResponse> {
        STATE
            .load(storage)
            .map_err(Into::into)
            .and_then(|state: State| {
                state.config().map(|config: &Config| ConfigResponse {
                    cadence_hours: config.cadence_hours(),
                })
            })
    }
}
