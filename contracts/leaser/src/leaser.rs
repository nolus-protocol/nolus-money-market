use std::collections::HashSet;

use currency::native::Nls;
use finance::{currency::SymbolOwned, liability::Liability, percent::Percent};
use lease::api::{ConnectionParams, DownpaymentCoin, InterestPaymentSpec};
use lpp::{msg::ExecuteMsg, stub::LppRef};
use oracle::stub::OracleRef;
use platform::batch::Batch;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Deps, StdResult, Storage};

use crate::{
    cmd::Quote,
    error::ContractError,
    migrate::{self},
    msg::{ConfigResponse, QuoteResponse},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

pub struct Leaser<'a> {
    deps: Deps<'a>,
}

impl<'a> Leaser<'a> {
    pub fn new(deps: Deps<'a>) -> Self {
        Self { deps }
    }
    pub fn config(&self) -> ContractResult<ConfigResponse> {
        let config = Config::load(self.deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn customer_leases(&self, owner: Addr) -> StdResult<HashSet<Addr>> {
        Leases::get(self.deps.storage, owner)
    }

    pub fn quote(
        &self,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
        max_ltd: Option<Percent>,
    ) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(self.deps.storage)?;

        let lpp = LppRef::try_new(config.lpp_addr, &self.deps.querier)?;

        let oracle = OracleRef::try_from(config.market_price_oracle, &self.deps.querier)?;

        let resp = lpp.execute_lender(
            Quote::new(
                self.deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.liability,
                config.lease_interest_rate_margin,
                max_ltd,
            ),
            &self.deps.querier,
        )?;

        Ok(resp)
    }
}

pub(super) fn try_setup_dex(
    storage: &mut dyn Storage,
    params: ConnectionParams,
) -> ContractResult<MessageResponse> {
    Config::setup_dex(storage, params)?;

    Ok(Default::default())
}

pub(super) fn try_configure(
    storage: &mut dyn Storage,
    lease_interest_rate_margin: Percent,
    liability: Liability,
    lease_interest_payment: InterestPaymentSpec,
) -> ContractResult<MessageResponse> {
    Config::update(
        storage,
        lease_interest_rate_margin,
        liability,
        lease_interest_payment,
    )?;

    Ok(Default::default())
}

pub(super) fn try_migrate_leases(
    storage: &mut dyn Storage,
    new_code_id: u64,
) -> ContractResult<MessageResponse> {
    Config::update_lease_code(storage, new_code_id)?;

    migrate::migrate_leases(Leases::iter(storage), new_code_id)
        .and_then(|batch| update_lpp(storage, new_code_id, batch))
}

pub(super) fn update_lpp(
    storage: &mut dyn Storage,
    new_code_id: u64,
    mut batch: Batch,
) -> ContractResult<MessageResponse> {
    let lpp = Config::load(storage)?.lpp_addr;
    let lpp_update_code = ExecuteMsg::NewLeaseCode {
        lease_code_id: new_code_id.into(),
    };
    batch
        .schedule_execute_wasm_no_reply::<_, Nls>(&lpp, lpp_update_code, None)
        .map(|()| batch.into())
        .map_err(Into::into)
}
