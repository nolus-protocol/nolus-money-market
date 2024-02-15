use currency::SymbolOwned;
use finance::percent::Percent;
use lease::api::open::{LoanForm, NewLeaseContract, NewLeaseForm};
use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::{
    cosmwasm_ext::as_dyn::storage,
    cosmwasm_std::{Addr, Coin},
};

use crate::{
    state::{config::Config, leases::Leases},
    ContractError,
};

use super::Borrow;

impl Borrow {
    pub fn with<S>(
        storage: &mut S,
        amount: Vec<Coin>,
        customer: Addr,
        admin: Addr,
        finalizer: Addr,
        currency: SymbolOwned,
        max_ltd: Option<Percent>,
    ) -> Result<MessageResponse, ContractError>
    where
        S: storage::DynMut + ?Sized,
    {
        Leases::cache_open_req(storage, &customer)
            .and_then(|()| Config::load(storage))
            .and_then(|config| {
                let mut batch = Batch::default();
                batch
                    .schedule_instantiate_wasm_reply_on_success(
                        config.lease_code_id,
                        &Self::open_lease_msg(customer, config, currency, max_ltd, finalizer),
                        Some(amount),
                        "lease".into(),
                        Some(admin), // allows lease migrations from this contract
                        Default::default(),
                    )
                    .map(|()| batch)
                    .map_err(Into::into)
            })
            .map(Into::into)
    }

    pub(crate) fn open_lease_msg(
        customer: Addr,
        config: Config,
        currency: SymbolOwned,
        max_ltd: Option<Percent>,
        finalizer: Addr,
    ) -> NewLeaseContract {
        NewLeaseContract {
            form: NewLeaseForm {
                customer,
                currency,
                max_ltd,
                position_spec: config.lease_position_spec,
                loan: LoanForm {
                    lpp: config.lpp,
                    profit: config.profit,
                    annual_margin_interest: config.lease_interest_rate_margin,
                    due_period: config.lease_due_period,
                },
                time_alarms: config.time_alarms,
                market_price_oracle: config.market_price_oracle,
            },
            dex: config.dex,
            finalizer,
        }
    }
}
