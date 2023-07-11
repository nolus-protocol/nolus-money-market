use std::ops::DerefMut;

use access_control::SingleUserAccess;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Deps, DepsMut, MessageInfo, Uint64};

use crate::{borrow::InterestRate, error::Result, state::Config};

pub(super) fn try_update_lease_code(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    lease_code: Uint64,
) -> Result<MessageResponse> {
    SingleUserAccess::new(
        deps.storage.deref_mut(),
        crate::access_control::LEASE_CODE_ADMIN_KEY,
    )
    .check(&info.sender)?;

    Config::update_lease_code(deps.storage, lease_code).map(|()| Default::default())
}

pub(super) fn try_update_parameters(
    deps: DepsMut<'_>,
    interest_rate: InterestRate,
) -> Result<MessageResponse> {
    Config::update_borrow_rate(deps.storage, interest_rate).map(|()| Default::default())
}

pub(super) fn query_config(deps: &Deps<'_>) -> Result<Config> {
    Config::load(deps.storage)
}
