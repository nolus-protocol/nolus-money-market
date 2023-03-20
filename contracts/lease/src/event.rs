pub enum Type {
    RequestLoan,
    OpenIcaAccount,
    OpeningSwap,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    RepaymentTransferIn,
    PaidActive,
    ClosingTransferIn,
    Closed,
    LiquidationWarning,
    Liquidation,
}

impl Type {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RequestLoan => "ls-request-loan",
            Self::OpenIcaAccount => "ls-open-dex-account",
            Self::OpeningSwap => "ls-open-swap",
            Self::OpenedActive => "ls-open",
            Self::RepaymentTransferOut => "ls-repay-transfer-out",
            Self::BuyLpn => "ls-repay-buy-lpn",
            Self::RepaymentTransferIn => "ls-repay-transfer-in",
            Self::PaidActive => "ls-repay",
            Self::ClosingTransferIn => "ls-close-transfer-in",
            Self::Closed => "ls-close",
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
