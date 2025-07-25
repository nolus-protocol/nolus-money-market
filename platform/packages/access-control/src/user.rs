use sdk::cosmwasm_std::{Addr, MessageInfo};

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
