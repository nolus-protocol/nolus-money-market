

/// 'wasm-' is always prepended by the runtime
pub const TYPES: [&str; 2] = [
    "lp-deposit",
    "lp-withdraw",
];

pub enum TYPE {
    Deposit,
    Withdraw,
}

