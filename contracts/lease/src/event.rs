use platform::batch::{Batch, Emit};

pub enum TYPE {
    Open,
    Close,
    Repay,
    // Liquidation,
}

impl TYPE {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "ls-open",
            Self::Close => "ls-close",
            TYPE::Repay => "ls-repay",
            // TYPE::Liquidation => "ls-liquidation",
        }
    }
}

impl From<TYPE> for String {
    fn from(ty: TYPE) -> Self {
        String::from(ty.as_str())
    }
}

pub fn emit_addr<K, V>(batch: Batch, ty: TYPE, event_key: K, event_value: V) -> Batch
where
    K: Into<String>,
    V: Into<String>,
{
    batch.emit(ty, event_key, event_value)
}
