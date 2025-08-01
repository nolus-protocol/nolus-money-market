use sdk::cosmwasm_std::{Addr, MessageInfo};

/// User is an abstraction to be used by access control checks
/// each struct that implements it can be used to be checked against for necessary permissions
pub trait User {
    fn addr(&self) -> &Addr;
}

impl User for MessageInfo {
    fn addr(&self) -> &Addr {
        &self.sender
    }
}

impl User for Addr {
    fn addr(&self) -> &Addr {
        self
    }
}
