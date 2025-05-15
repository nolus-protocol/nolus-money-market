use sdk::cosmwasm_std::Addr;

use crate::{ContractError, msg::Config, result::ContractResult};

pub type AnomalyResolutionPermission<'config> = LeaseAdminOnly<'config>;
pub type ChangeLeaseAdminPermission<'config> = LeaseAdminOnly<'config>;
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

#[cfg(all(feature = "internal.test.testing", test))]
mod tests {
    use sdk::cosmwasm_std::Addr;

    use crate::{ContractError, authz::LeaseAdminOnly, tests};

    #[test]
    fn check_fail() {
        let config = tests::config();
        let access = LeaseAdminOnly::from(&config);
        let not_authorized = Addr::unchecked("hacker");

        assert!(matches!(
            access.check_access(&not_authorized).unwrap_err(),
            ContractError::Unauthorized(_)
        ));
    }

    #[test]
    fn check_pass() {
        let config = tests::config();
        let access = LeaseAdminOnly::from(&config);

        assert_eq!(Ok(()), access.check_access(&config.lease_admin));
    }
}
