use finance::{coin::Coin, currency::Currency};
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::nlpn::NLpn;

pub fn emit_deposit<LPN>(
    env: Env,
    lender_addr: Addr,
    deposited_amount: Coin<LPN>,
    receipts: Coin<NLpn>,
) -> Emitter
where
    LPN: Currency,
{
    Emitter::of_type("lp-deposit")
        .emit_tx_info(&env)
        .emit("from", lender_addr)
        .emit("to", env.contract.address)
        .emit_coin("deposit", deposited_amount)
        .emit_coin_amount("receipts", receipts)
}

pub fn emit_withdraw<LPN>(
    env: Env,
    lender_addr: Addr,
    payment_lpn: Coin<LPN>,
    receipts: Coin<NLpn>,
    close_flag: bool,
) -> Emitter
where
    LPN: Currency,
{
    Emitter::of_type("lp-withdraw")
        .emit_tx_info(&env)
        .emit("to", lender_addr)
        .emit("from", env.contract.address)
        .emit_coin("withdraw", payment_lpn)
        .emit_coin_amount("receipts", receipts)
        .emit_to_string_value("close", close_flag)
}
