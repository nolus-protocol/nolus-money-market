#[cfg(feature = "cosmos_proto")]
pub use cosmos_sdk_proto;
#[cfg(feature = "schema")]
pub use cosmwasm_schema::{self, schemars};
#[cfg(feature = "cosmos")]
pub use cosmwasm_std;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use cw_multi_test;
#[cfg(feature = "storage")]
pub use cw_storage_plus;
#[cfg(feature = "neutron")]
pub use neutron_sdk;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod testing;

pub mod cosmwasm_ext {
    #[cfg(not(feature = "neutron"))]
    pub use cosmwasm_std::Empty as InterChainMsg;
    #[cfg(feature = "neutron")]
    pub use neutron_sdk::bindings::msg::NeutronMsg as InterChainMsg;

    pub type Response = cosmwasm_std::Response<InterChainMsg>;
    pub type CosmosMsg = cosmwasm_std::CosmosMsg<InterChainMsg>;
    pub type SubMsg = cosmwasm_std::SubMsg<InterChainMsg>;
}
