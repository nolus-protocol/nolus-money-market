use platform::access_control::AccessControl;

pub(crate) static OWNER: AccessControl = AccessControl::new("contract_owner");
pub(crate) static REWARDS_DISPATCHER: AccessControl =
    AccessControl::new("contract_rewards_dispatcher");
