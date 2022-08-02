#[derive(Clone)]
pub enum Type {
    Deposit,
    Withdraw,
}

impl Type {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "lp-deposit",
            Self::Withdraw => "lp-withdraw",
        }
    }
}

impl From<Type> for String {
    fn from(ty: Type) -> Self {
        String::from(ty.as_str())
    }
}
