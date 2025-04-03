pub enum Type {
    RequestLoan,
    OpenIcaAccount,
    OpeningSwap,
    OpenedActive,
    RepaymentSwap,
    PaidActive,
    ClosingTransferIn,
    Closed,
    LiquidationWarning,
    LiquidationSwap,
    Liquidation,
    ClosePosition,
    AutoClosePosition,
    SlippageAnomaly,
}

impl Type {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RequestLoan => "ls-request-loan",
            Self::OpenIcaAccount => "ls-open-dex-account",
            Self::OpeningSwap => "ls-open-swap",
            Self::OpenedActive => "ls-open",
            Self::RepaymentSwap => "ls-repay-swap",
            Self::PaidActive => "ls-repay",
            Self::ClosingTransferIn => "ls-close-transfer-in",
            Self::Closed => "ls-close",
            Self::LiquidationWarning => "ls-liquidation-warning",
            Self::LiquidationSwap => "ls-liquidation-swap",
            Self::Liquidation => "ls-liquidation",
            Self::ClosePosition => "ls-close-position",
            Self::AutoClosePosition => "ls-auto-close-position",
            Self::SlippageAnomaly => "ls-slippage-anomaly",
        }
    }
}

impl From<Type> for String {
    fn from(ty: Type) -> Self {
        String::from(ty.as_str())
    }
}
