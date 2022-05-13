use cosmwasm_std::Addr;

use crate::msg::{Liability, Repayment};

pub fn leaser_instantiate_msg(lease_code_id: u64, lpp_addr: Addr) -> crate::msg::InstantiateMsg {
    crate::msg::InstantiateMsg {
        lease_code_id,
        lpp_ust_addr: lpp_addr,
        lease_interest_rate_margin: 3,
        liability: Liability::new(65, 70, 80),
        repayment: Repayment::new(90 * 24 * 60 * 60, 10 * 24 * 60 * 60),
    }
}
