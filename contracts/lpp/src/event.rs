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
    let transaction_idx = env.transaction.expect("Error! No transaction index.");

    batch
        .into_emitter("lp-deposit")
        .emit_to_string_value("height", env.block.height)
        .emit_to_string_value("idx", transaction_idx.index)
        .emit("from", lender_addr)
        .emit_timestamp("at", &env.block.time)
        .emit("to", env.contract.address)
        .emit_coin("deposit", deposited_amount)
        .emit_coin_amount("receipts", receipts)
}
