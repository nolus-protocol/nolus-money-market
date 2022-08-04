use cosmwasm_std::{Addr, Env};
use finance::{coin::Coin, currency::Currency};
use platform::batch::Batch;
use crate::nlpn::NLpn;

pub fn emit_withdraw<C>(
    mut batch: Batch,
    env: Env,
    lender_addr: Addr,
    payment_lpn: Coin<C>,
    receipts: Coin<NLpn>,
    close_flag: bool
) -> Batch
where
    C: Currency,
{
    const WITHDRAW:&str = "lp-withdraw";
    let transaction_idx = env.transaction.expect("Error! No transaction index.");

    batch.emit(WITHDRAW,"height" , env.block.height.to_string());
    batch.emit(WITHDRAW,"idx" , transaction_idx.index.to_string());
    batch.emit(WITHDRAW,"to" , lender_addr);
    batch.emit_timestamp(WITHDRAW,"at" ,  &env.block.time);
    batch.emit(WITHDRAW,"from" ,  env.contract.address);
    batch.emit_coin(WITHDRAW,"withdraw", payment_lpn);
    batch.emit_amount(WITHDRAW ,"receipts" , receipts);
    batch.emit(WITHDRAW,"close" , close_flag.to_string());
    batch
}
