pub enum Type {
    Open,
    Close,
    Repay,
    LiquidationWarning,
    Liquidation,
}

impl Type {
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

impl From<Type> for String {
    fn from(ty: Type) -> Self {
        String::from(ty.as_str())
    }
}
