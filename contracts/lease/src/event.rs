pub enum TYPE {
    Open,
    Close,
    Repay,
    LiquidationWarning,
    Liquidation,
}

impl TYPE {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "ls-open",
            Self::Close => "ls-close",
            Self::Repay => "ls-repay",
            Self::LiquidationWarning => "ls-liquidation-warning",
            Self::Liquidation => "ls-liquidation",
        }
    }
}

impl From<TYPE> for String {
    fn from(ty: TYPE) -> Self {
        String::from(ty.as_str())
    }
}
