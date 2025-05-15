use sdk::cosmwasm_std::Addr;

use crate::{ContractError, msg::Config, result::ContractResult};

pub type AnomalyResolutionPermission<'config> = LeaseAdminOnly<'config>;
pub type LeasesConfigurationPermission<'config> = LeaseAdminOnly<'config>;

pub struct LeaseAdminOnly<'config>(&'config Config);
impl<'config> LeaseAdminOnly<'config> {
    pub fn from(config: &'config Config) -> Self {
        Self(config)
    }

    pub fn granted_to(&self, caller: &Addr) -> bool {
        caller == self.0.lease_admin
    }

    // TODO issue#70
    pub fn check_access(&self, caller: &Addr) -> ContractResult<()> {
        if self.granted_to(caller) {
            Ok(())
        } else {
            Err(ContractError::Unauthorized(
                access_control::error::Error::Unauthorized {},
            ))
        }
    }
}
