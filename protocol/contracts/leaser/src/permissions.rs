use access_control::{AccessPermission, error::Error as AccessControlError, user::User};
use lease::api::authz::AccessGranted;
use sdk::cosmwasm_std::Addr;

use crate::{ContractError, result::ContractResult, state::config::Config};

pub struct LeaseAdminOnly<'a> {
    lease_config: &'a Config,
}

impl<'a> LeaseAdminOnly<'a> {
    pub fn new(lease_config: &'a Config) -> Self {
        Self { lease_config }
    }

    pub fn check_permission(&self, caller: &Addr) -> ContractResult<AccessGranted> {
        self.granted_to(caller)
            .map_err(ContractError::CheckPermission)
            .map(|granted| {
                if granted {
                    AccessGranted::Yes
                } else {
                    AccessGranted::No
                }
            })
    }
}

impl AccessPermission for LeaseAdminOnly<'_> {
    fn granted_to<U>(&self, user: &U) -> Result<bool, AccessControlError>
    where
        U: User,
    {
        Ok(self.lease_config.lease_admin == user.addr())
    }
}

pub type LeasesConfigurationPermission<'a> = LeaseAdminOnly<'a>;
pub type ChangeLeaseAdminPermission<'a> = LeaseAdminOnly<'a>;
pub type AnomalyResolutionPermission<'a> = LeaseAdminOnly<'a>;
pub type ClosePositionPermission<'a> = LeaseAdminOnly<'a>;
