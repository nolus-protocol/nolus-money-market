use cosmwasm_std::{Addr, Env};
use finance::{coin::Coin, currency::Currency};
use platform::batch::Batch;
use crate::nlpn::NLpn;

const DEPOSIT:&str = "lp-deposit";

pub fn emit_deposit<C>(
    mut batch: Batch,
    env: Env,
    lender_addr: Addr,
    deposited_amount: Coin<C>,
    receipts: Coin<NLpn>,
) -> Batch
where
    C: Currency,
{
    let transaction_idx = env.transaction.expect("Error! No transaction index.");

    batch.emit(DEPOSIT, "height", env.block.height.to_string());
    batch.emit(DEPOSIT, "idx", transaction_idx.index.to_string());
    batch.emit(DEPOSIT, "from", lender_addr);
    batch.emit_timestamp(DEPOSIT, "at", &env.block.time);
    batch.emit(DEPOSIT, "to", env.contract.address);
    batch.emit_coin(DEPOSIT, "deposit", deposited_amount);
    batch.emit_amount(DEPOSIT, "receipts", receipts);
    batch
}
