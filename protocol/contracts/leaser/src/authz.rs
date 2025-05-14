use sdk::cosmwasm_std::Addr;

use crate::msg::Config;

pub struct AnomalyResolutionPermission<'config> {
    config: &'config Config,
}

impl<'config> AnomalyResolutionPermission<'config> {
    pub fn from(config: &'config Config) -> Self {
        Self { config }
    }

    pub fn granted_to(&self, caller: &Addr) -> bool {
        caller == self.config.lease_admin
    }
}
