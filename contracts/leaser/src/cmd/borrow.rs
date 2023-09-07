use currency::SymbolOwned;
use finance::percent::Percent;
use lease::api::{LoanForm, NewLeaseContract, NewLeaseForm};
use platform::batch::Batch;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Coin, DepsMut};

use crate::{
    result::ContractResult,
    state::{config::Config, leases::Leases},
    ContractError,
};

use super::Borrow;

impl Borrow {
    pub fn with(
        deps: DepsMut<'_>,
        amount: Vec<Coin>,
        customer: Addr,
        admin: Addr,
        finalizer: Addr,
        currency: SymbolOwned,
        max_ltd: Option<Percent>,
    ) -> Result<MessageResponse, ContractError> {
        let config = Config::load(deps.storage)?;
        let instance_reply_id = Leases::cache_open_req(deps.storage, &customer)?;

        let mut batch = Batch::default();
        batch.schedule_instantiate_wasm_on_success_reply(
            config.lease_code_id,
            Self::open_lease_msg(customer, config, currency, max_ltd, finalizer)?,
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
        max_ltd: Option<Percent>,
        finalizer: Addr,
    ) -> ContractResult<NewLeaseContract> {
        config
            .dex
            .map(|dex| NewLeaseContract {
                form: NewLeaseForm {
                    customer,
                    currency,
                    max_ltd,
                    position_spec: config.lease_position_spec,
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
                finalizer,
            })
            .ok_or(ContractError::NoDEXConnectivitySetup {})
    }
}
