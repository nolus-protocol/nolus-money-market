use platform::batch::Batch;

/// 'wasm-' is always prepended by the runtime
const TYPES: [&str; 4] = [
    "ls-open",
    "ls-close",
    "ls-repay",
    "ls-liquidation",
];
pub enum TYPE {
    Open = 0,
    // Close,
    // Repay,
    // Liquidation,
}

pub fn emit_addr<K, V>(mut batch: Batch, ty: TYPE, event_key: K, event_value: V) -> Batch
where
    K: Into<String>,
    V: Into<String>,
{
    batch.emit(TYPES[ty as usize], event_key, event_value);
    batch
}
