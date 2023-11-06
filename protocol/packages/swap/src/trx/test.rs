#[cfg(dex = "osmosis")]
mod msg {

    pub type RequestMsg = crate::trx::osmosis::RequestMsg;

    pub type ResponseMsg = crate::trx::osmosis::ResponseMsg;
}
pub use msg::*;
