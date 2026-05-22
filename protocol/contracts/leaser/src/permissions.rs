use std::marker::PhantomData;

use access_control::{AccessPermission, error::Error as AccessControlError, user::User};
use lease::api::authz::AccessGranted;
use sdk::cosmwasm_std::Addr;

use crate::{ContractError, result::ContractResult, state::config::Config};

pub type LeasesConfigurationPermission<'a> = LeaseAdminOnly<'a>;
pub type ChangeLeaseAdminPermission<'a> = LeaseAdminOnly<'a>;
pub type AnomalyResolutionPermission<'a> = LeaseAdminOnly<'a>;
pub type ClosePositionPermission<'a> = LeaseAdminOnly<'a>;
pub type LeaseAdminOnly<'a> = ConfigAddrPermission<'a, LeaseAdmin>;

/// Authorises `ExecuteMsg::RemoteLeaseCallback` dispatched to a lease — granted
/// only to the local remote-lease controller recorded in `Config.remote_lease_controller`.
pub type RemoteLeaseCallbackPermission<'a> = ConfigAddrPermission<'a, RemoteLeaseController>;

/// Selects which `Addr` in the leaser's [`Config`] gates a given permission.
///
/// Each topic (lease-admin operations, remote-lease callback, …) is a
/// zero-sized marker that points at one of the addresses stored in the
/// configuration. Used as the second generic parameter of
/// [`ConfigAddrPermission`].
pub trait ConfigAddrSelector {
    fn select(config: &Config) -> &Addr;
}

/// Permission granted to a caller whose address matches the leaser
/// [`Config`] field picked by `F`.
pub struct ConfigAddrPermission<'a, F> {
    lease_config: &'a Config,
    _selector: PhantomData<F>,
}

impl<'a, F> ConfigAddrPermission<'a, F> {
    pub const fn new(lease_config: &'a Config) -> Self {
        Self {
            lease_config,
            _selector: PhantomData,
        }
    }

    pub fn check_permission(&self, caller: &Addr) -> ContractResult<AccessGranted>
    where
        F: ConfigAddrSelector,
    {
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

impl<F> AccessPermission for ConfigAddrPermission<'_, F>
where
    F: ConfigAddrSelector,
{
    fn granted_to<U>(&self, user: &U) -> Result<bool, AccessControlError>
    where
        U: User,
    {
        Ok(F::select(self.lease_config) == user.addr())
    }
}

pub struct LeaseAdmin;
impl ConfigAddrSelector for LeaseAdmin {
    fn select(config: &Config) -> &Addr {
        &config.lease_admin
    }
}

pub struct RemoteLeaseController;
impl ConfigAddrSelector for RemoteLeaseController {
    fn select(config: &Config) -> &Addr {
        &config.remote_lease_controller
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use sdk::cosmwasm_std::Addr;

    use lease::api::authz::AccessGranted;

    use crate::tests;

    use super::RemoteLeaseCallbackPermission;

    #[test]
    fn matching_caller_is_granted() {
        let config = tests::config();
        let caller = config.remote_lease_controller.clone();
        let granted = RemoteLeaseCallbackPermission::new(&config)
            .check_permission(&caller)
            .expect("permission check must not error");
        assert_eq!(AccessGranted::Yes, granted);
    }

    #[test]
    fn mismatched_caller_is_denied() {
        let config = tests::config();
        let granted = RemoteLeaseCallbackPermission::new(&config)
            .check_permission(&Addr::unchecked("not the controller"))
            .expect("permission check must not error");
        assert_eq!(AccessGranted::No, granted);
    }
}
