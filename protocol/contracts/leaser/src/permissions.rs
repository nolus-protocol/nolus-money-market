use access_control::{AccessPermission, user::User};

use crate::state::config::Config;

pub struct LeaseAdminOnly<'a> {
    lease_config: &'a Config,
}

impl<'a> LeaseAdminOnly<'a> {
    pub fn new(lease_config: &'a Config) -> Self {
        Self { lease_config }
    }
}

impl AccessPermission for LeaseAdminOnly<'_> {
    fn granted_to<S>(&self, user: &U) -> bool
    where
        U: User,
    {
        self.lease_config.lease_admin == user.addr()
    }
}

pub type LeasesConfigurationPermission<'a> = LeaseAdminOnly<'a>;
pub type ChangeLeaseAdminPermission<'a> = LeaseAdminOnly<'a>;
pub type AnomalyResolutionPermission<'a> = LeaseAdminOnly<'a>;
