use platform::access_control::AccessControl;

pub(crate) static OWNER: AccessControl = AccessControl::new("contract_owner");
pub(crate) static TIMEALARMS: AccessControl = AccessControl::new("contract_timealarms");
