pub enum Type {
    RequestLoan,
    OpeningSwap,
    OpeningUnwind,
    OpenedActive,
    RepaymentSwap,
    RepaymentTransferOut,
    PaidActive,
    ClosingTransferOut,
    ClosingRemoteLease,
    Closed,
    LiquidationWarning,
    LiquidationSwap,
    Liquidation,
    ClosePosition,
    AutoClosePosition,
}

impl Type {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RequestLoan => "ls-request-loan",
            Self::OpeningSwap => "ls-open-swap",
            Self::OpeningUnwind => "ls-open-unwind",
            Self::OpenedActive => "ls-open",
            Self::RepaymentSwap => "ls-repay-swap",
            Self::RepaymentTransferOut => "ls-repay-transfer-out",
            Self::PaidActive => "ls-repay",
            Self::ClosingTransferOut => "ls-close-transfer-out",
            Self::ClosingRemoteLease => "ls-close-remote-lease",
            Self::Closed => "ls-close",
            Self::LiquidationWarning => "ls-liquidation-warning",
            Self::LiquidationSwap => "ls-liquidation-swap",
            Self::Liquidation => "ls-liquidation",
            Self::ClosePosition => "ls-close-position",
            Self::AutoClosePosition => "ls-auto-close-position",
        }
    }
}

impl From<Type> for String {
    fn from(ty: Type) -> Self {
        String::from(ty.as_str())
    }
}
