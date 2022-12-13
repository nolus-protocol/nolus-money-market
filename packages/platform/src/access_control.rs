use cw_controllers::{Admin, AdminError};

use sdk::cosmwasm_std::{Addr, CustomQuery, Deps, DepsMut, StdError, StdResult};

pub struct AccessControl(Admin<'static>);

impl AccessControl {
    pub const fn new(namespace: &'static str) -> Self {
        Self(Admin::new(namespace))
    }

    pub fn get_address<Q, E>(&self, deps: Deps<Q>) -> Result<Addr, E>
    where
        Q: CustomQuery,
        E: From<StdError> + From<NotSet>,
    {
        self.0.get(deps)?.ok_or_else(|| NotSet.into())
    }

    pub fn set_address<Q>(&self, deps: DepsMut<Q>, addr: Addr) -> StdResult<()>
    where
        Q: CustomQuery,
    {
        self.0.set(deps, Some(addr))
    }

    pub fn assert_address<Q, E>(&self, deps: Deps<Q>, addr: &Addr) -> Result<(), E>
    where
        Q: CustomQuery,
        E: From<StdError> + From<Unauthorized>,
    {
        self.0
            .assert_admin(deps, addr)
            .map_err(|error| match error {
                AdminError::Std(error) => error.into(),
                AdminError::NotAdmin { .. } => Unauthorized.into(),
            })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
#[error("[Platform~Access Control] Access control variable not associated with any address!")]
pub struct NotSet;

#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
#[error("[Platform~Access Control] Checked address doesn't match the one associated with access control variable!")]
pub struct Unauthorized;
