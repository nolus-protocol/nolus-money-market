#[cfg(feature = "schema")]
pub use cosmwasm_schema::{self, schemars};
#[cfg(feature = "cosmos")]
pub use cosmwasm_std;
#[cfg(feature = "storage")]
pub use cosmwasm_storage;
#[cfg(feature = "contract")]
pub use cw2;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use cw_multi_test;
#[cfg(feature = "storage")]
pub use cw_storage_plus;
#[cfg(feature = "neutron")]
pub use neutron_sdk;

#[cfg(feature = "testing")]
pub mod testing;

pub mod cosmwasm_ext {
    #[cfg(not(feature = "neutron"))]
    pub use cosmwasm_std::{CosmosMsg, Response, SubMsg};

    #[cfg(feature = "neutron")]
    pub type Response = cosmwasm_std::Response<neutron_sdk::bindings::msg::NeutronMsg>;

    #[cfg(feature = "neutron")]
    pub type CosmosMsg = cosmwasm_std::CosmosMsg<neutron_sdk::bindings::msg::NeutronMsg>;

    #[cfg(feature = "neutron")]
    pub type SubMsg = cosmwasm_std::SubMsg<neutron_sdk::bindings::msg::NeutronMsg>;
}
