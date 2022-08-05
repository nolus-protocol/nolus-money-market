use crate::nlpn::NLpn;
use cosmwasm_std::{Addr, Env};
use finance::{coin::Coin, currency::Currency};
use platform::batch::{Batch, Emit, Emitter};

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
    let transaction_idx = env.transaction.expect("Error! No transaction index.");


    batch
    .into_emitter("lp-withdraw")
    .emit_to_string_value("height", env.block.height)
    .emit_to_string_value("idx",  transaction_idx.index)
    .emit("to", lender_addr)
    .emit_timestamp("at", &env.block.time)
    .emit("from", env.contract.address)
    .emit_coin("withdraw", payment_lpn)
    .emit_coin_amount("receipts",  receipts)
    .emit_to_string_value("close", close_flag)

}
