#[cfg(feature = "astroport")]
mod msg {

    pub type RequestMsg = crate::trx::astroport::RequestMsg;

    pub type ResponseMsg = crate::trx::astroport::ResponseMsg;
}
#[cfg(all(not(feature = "astroport"), feature = "osmosis"))]
mod msg {

    pub type RequestMsg = crate::trx::osmosis::RequestMsg;

    pub type ResponseMsg = crate::trx::osmosis::ResponseMsg;
}pub use msg::*;
