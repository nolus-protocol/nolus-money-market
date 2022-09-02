use cosmwasm_std::{Addr, Coin, DepsMut, Response};

use finance::currency::SymbolOwned;
use lease::msg::{LoanForm, NewLeaseForm};
use platform::batch::Batch;

use crate::{
    state::{config::Config, leaser::Loans},
    ContractError,
};

use super::Borrow;

impl Borrow {
    pub fn with(
        deps: DepsMut,
        amount: Vec<Coin>,
        sender: Addr,
        currency: SymbolOwned,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        let instance_reply_id = Loans::next(deps.storage, sender.clone())?;

        let mut batch = Batch::default();
        batch.schedule_instantiate_wasm_on_success_reply(
            config.lease_code_id,
            Self::open_lease_msg(sender, config, currency),
            Some(amount),
            "lease",
            None,
            instance_reply_id,
        )?;
        Ok(batch.into())
    }

    pub(crate) fn open_lease_msg(
        sender: Addr,
        config: Config,
        currency: SymbolOwned,
    ) -> NewLeaseForm {
        NewLeaseForm {
            customer: sender.into_string(),
            currency,
            liability: config.liability,
            loan: LoanForm {
                annual_margin_interest: config.lease_interest_rate_margin,
                lpp: config.lpp_addr.into_string(),
                interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
                grace_period_secs: config.repayment.grace_period_sec,
            },
            time_alarms: config.time_alarms,
            market_price_oracle: config.market_price_oracle,
        }
    }
}
