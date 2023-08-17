use finance::percent::BoundToHundredPercent;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Storage, Uint64};

use crate::{borrow::InterestRate, error::Result, state::Config};

pub(super) fn try_update_lease_code(
    storage: &mut dyn Storage,
    lease_code: Uint64,
) -> Result<MessageResponse> {
    Config::update_lease_code(storage, lease_code).map(|()| MessageResponse::default())
}

pub(super) fn try_update_borrow_rate(
    storage: &mut dyn Storage,
    borrow_rate: InterestRate,
) -> Result<MessageResponse> {
    Config::update_borrow_rate(storage, borrow_rate).map(|()| MessageResponse::default())
}

pub(super) fn try_update_min_utilization(
    storage: &mut dyn Storage,
    min_utilization: BoundToHundredPercent,
) -> Result<MessageResponse> {
    Config::update_min_utilization(storage, min_utilization).map(|()| MessageResponse::default())
}

pub(super) fn query_config(storage: &dyn Storage) -> Result<Config> {
    Config::load(storage)
}
