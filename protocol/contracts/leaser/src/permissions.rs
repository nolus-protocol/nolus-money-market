use access_control::{AccessPermission, sender::SenderAssurance};

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
    fn granted_to<S>(&self, sender: &S) -> bool
    where
        S: SenderAssurance,
    {
        self.lease_config.lease_admin == sender.as_ref()
    }
}

pub type LeasesConfigurationPermission<'a> = LeaseAdminOnly<'a>;
pub type ChangeLeaseAdminPermission<'a> = LeaseAdminOnly<'a>;
pub type AnomalyResolutionPermission<'a> = LeaseAdminOnly<'a>;
