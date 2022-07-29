#[derive(Clone)]
pub enum Event {
    Deposit,
    Withdraw,
}

impl Event {
    /// 'wasm-' is always prepended by the runtime
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "lp-deposit",
            Self::Withdraw => "lp-withdraw",
        }
    }
}

impl From<Event> for String {
    fn from(ty: Event) -> Self {
        String::from(ty.as_str())
    }
}
