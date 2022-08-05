pub enum TYPE {
    Open,
    Close,
    // Repay,
    // Liquidation,
}

impl TYPE {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "ls-open",
            Self::Close => "ls-close",
            // TYPE::Repay => "ls-repay",
            // TYPE::Liquidation => "ls-liquidation",
        }
    }
}

impl From<TYPE> for String {
    fn from(ty: TYPE) -> Self {
        String::from(ty.as_str())
    }
}
