use cosmwasm_std::Env;
use finance::{coin::Coin, currency::Currency};
use platform::batch::{Batch, Emit, Emitter};

pub fn emit_profit<C>(
    batch: Batch,
    env: Env,
    deposited_amount: Coin<C>,
    // receipts: Coin<NLpn>,
) -> Emitter
where
    C: Currency,
{
    let transaction_idx = env.transaction.expect("Error! No transaction index.");

    batch
        .into_emitter("tr-profit")
        .emit_to_string_value("height", env.block.height)
        .emit_to_string_value("idx", transaction_idx.index)
        .emit_timestamp("at", &env.block.time)
        .emit_coin("amount", deposited_amount)
    //TODO: -in-stable
    // .emit_coin("amount", deposited_amount)
}
