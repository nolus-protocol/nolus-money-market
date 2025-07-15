use access_control::{AccessPermission, permissions::SingleUserPermission};
use currency::{Currency, Group, MemberOf};
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::MessageInfo;

struct LeaseAdminOnly<'a> {
    lease_config: &'a Config,
}

impl<'a> LeaseAdminOnly<'a> {
    pub fn new(lease_config: &'a Config) -> Self {
        Self { lease_config }
    }
}

impl AccessPermission for LeaseAdminOnly<'_> {
    fn granted_to(&self, info: &MessageInfo) -> bool {
        self.lease_config.lease_admin == info.sender
    }
}

pub type LeasesConfigurationPermission<'a> = LeaseAdminOnly<'a>;
pub type ChangeLeaseAdminPermission<'a> = LeaseAdminOnly<'a>;
pub type AnomalyResolutionPermission<'a> = LeaseAdminOnly<'a>;
