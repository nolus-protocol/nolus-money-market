pub enum TYPE {
    Deposit,
    Withdraw,
}

impl TYPE {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "lp-deposit",
            Self::Withdraw => "lp-withdraw",
        }
    }
}

impl From<TYPE> for String {
    fn from(ty: TYPE) -> Self {
        String::from(ty.as_str())
    }
}

impl Clone for TYPE {
    fn clone(&self) -> TYPE {
        match self {
            Self::Deposit => TYPE::Deposit,
            Self::Withdraw => TYPE::Withdraw,
        }
    }
}

