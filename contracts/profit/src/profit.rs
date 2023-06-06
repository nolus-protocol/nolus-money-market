use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::BankAccount,
    batch::{Emit as _, Emitter},
    message::Response as PlatformResponse,
};
use sdk::cosmwasm_std::{Addr, Env, Storage};

use crate::{
    msg::ConfigResponse,
    result::ContractResult,
    state::{ConfigManagement as _, State},
};

pub struct Profit;

impl Profit {
    pub(crate) fn transfer_nls<B>(
        mut account: B,
        env: &Env,
        treasury_addr: &Addr,
        balance_nls: Coin<Nls>,
    ) -> PlatformResponse
    where
        B: BankAccount,
    {
        if balance_nls.is_zero() {
            PlatformResponse::messages_only(account.into())
        } else {
            account.send(balance_nls, treasury_addr);

            PlatformResponse::messages_with_events(
                account.into(),
                Emitter::of_type("tr-profit")
                    .emit_tx_info(env)
                    .emit_coin("profit-amount", balance_nls),
            )
        }
    }

    pub fn query_config(storage: &dyn Storage) -> ContractResult<ConfigResponse> {
        State::load(storage).and_then(|state: State| state.try_query_config())
    }
}
