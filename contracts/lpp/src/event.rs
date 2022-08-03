use cosmwasm_std::{Addr, Env};
use finance::{coin::Coin, currency::Currency};
use platform::batch::Batch;
#[derive(Clone)]
pub enum Type {
    Deposit,
    Withdraw,
}

impl Type {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "lp-deposit",
            Self::Withdraw => "lp-withdraw",
        }
    }
}

impl From<Type> for String {
    fn from(ty: Type) -> Self {
        String::from(ty.as_str())
    }
}


pub fn emit_deposit<C,V>(mut batch: Batch,env: Env, lender_addr: Addr,deposited_amount: Coin<C>, receipts: Coin<V>) -> Batch
where
C: Currency,
V: Currency
{
    let transaction_idx = env.transaction.expect("Error! No transaction index.");

    batch.emit(Type::Deposit, "height", env.block.height.to_string());
    batch.emit(Type::Deposit, "idx", transaction_idx.index.to_string());
    batch.emit(Type::Deposit, "from", lender_addr);
    batch.emit_timestamp(Type::Deposit, "at", &env.block.time);
    batch.emit(Type::Deposit, "to", env.contract.address);
    batch.emit_coin(Type::Deposit,"deposit", deposited_amount);
    batch.emit_amount(Type::Deposit, "receipts", receipts);
    batch
}
