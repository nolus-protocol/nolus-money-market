#[cfg(not(feature = "neutron"))]
pub use cosmwasm_std::Empty as InterChainMsg;
#[cfg(feature = "neutron")]
pub use neutron_sdk::bindings::msg::NeutronMsg as InterChainMsg;

pub type Response = cosmwasm_std::Response<InterChainMsg>;
pub type CosmosMsg = cosmwasm_std::CosmosMsg<InterChainMsg>;
pub type SubMsg = cosmwasm_std::SubMsg<InterChainMsg>;

pub mod as_dyn;
