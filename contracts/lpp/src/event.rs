use crate::nlpn::NLpn;
use cosmwasm_std::{Addr, Env};
use finance::{coin::Coin, currency::Currency};
use platform::batch::{Batch, Emit, Emitter};

pub fn emit_deposit<C>(
    batch: Batch,
    env: Env,
    lender_addr: Addr,
    deposited_amount: Coin<C>,
    receipts: Coin<NLpn>,
) -> Emitter
where
    C: Currency,
{
    batch
        .into_emitter("lp-deposit")
        .emit_block_info(&env)
        .emit("from", lender_addr)
        .emit_timestamp("at", &env.block.time)
        .emit("to", env.contract.address)
        .emit_coin("deposit", deposited_amount)
        .emit_coin_amount("receipts", receipts)
}

pub fn emit_withdraw<C>(
    batch: Batch,
    env: Env,
    lender_addr: Addr,
    payment_lpn: Coin<C>,
    receipts: Coin<NLpn>,
    close_flag: bool,
) -> Emitter
where
    C: Currency,
{
    batch
        .into_emitter("lp-withdraw")
        .emit_block_info(&env)
        .emit("to", lender_addr)
        .emit_timestamp("at", &env.block.time)
        .emit("from", env.contract.address)
        .emit_coin("withdraw", payment_lpn)
        .emit_coin_amount("receipts", receipts)
        .emit_to_string_value("close", close_flag)
}
