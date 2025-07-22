use sdk::cosmwasm_std::{Addr, MessageInfo};

pub struct Sender<'a> {
    pub addr: &'a Addr,
}

pub trait SenderAssurance
where
    Self: AsRef<Addr>,
{
}

impl<'a> Sender<'a> {
    pub fn of_execute_msg(info: &'a MessageInfo) -> Self {
        Self { addr: &info.sender }
    }

    pub fn from_addr(addr: &'a Addr) -> Self {
        Self { addr }
    }
}

impl<'a> AsRef<Addr> for Sender<'a> {
    fn as_ref(&self) -> &Addr {
        self.addr
    }
}

impl<'a> SenderAssurance for Sender<'a> {}
