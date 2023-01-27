use finance::currency::SymbolOwned;
use lease::api::{LoanForm, NewLeaseContract, NewLeaseForm};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Coin, DepsMut},
};

use crate::{
    error::ContractResult,
    state::{config::Config, leases::Leases},
    ContractError,
};

use super::Borrow;

impl Borrow {
    pub fn with(
        deps: DepsMut,
        amount: Vec<Coin>,
        customer: Addr,
        admin: Addr,
        currency: SymbolOwned,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        let instance_reply_id = Leases::next(deps.storage, customer.clone())?;

        let mut batch = Batch::default();
        batch.schedule_instantiate_wasm_on_success_reply(
            config.lease_code_id,
            Self::open_lease_msg(customer, config, currency)?,
            Some(amount),
            "lease",
            Some(admin), // allows lease migrations from this contract
            instance_reply_id,
        )?;
        Ok(batch.into())
    }

    pub(crate) fn open_lease_msg(
        customer: Addr,
        config: Config,
        currency: SymbolOwned,
    ) -> ContractResult<NewLeaseContract> {
        config
            .dex
            .map(|dex| NewLeaseContract {
                form: NewLeaseForm {
                    customer,
                    currency,
                    liability: config.liability,
                    loan: LoanForm {
                        annual_margin_interest: config.lease_interest_rate_margin,
                        lpp: config.lpp_addr,
                        interest_payment: config.lease_interest_payment,
                        profit: config.profit,
                    },
                    time_alarms: config.time_alarms,
                    market_price_oracle: config.market_price_oracle,
                },
                dex,
            })
            .ok_or(ContractError::NoDEXConnectivitySetup {})
    }
}
