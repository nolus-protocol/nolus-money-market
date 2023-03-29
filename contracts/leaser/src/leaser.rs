use std::collections::HashSet;

use currency::native::Nls;
use finance::{currency::SymbolOwned, liability::Liability, percent::Percent};
use lease::api::{dex::ConnectionParams, DownpaymentCoin, InterestPaymentSpec};
use lpp::{msg::ExecuteMsg, stub::lender::LppLenderRef};
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, StdResult, Storage},
};

use crate::{
    cmd::Quote,
    error::{ContractError, ContractResult},
    migrate::{self},
    msg::{ConfigResponse, QuoteResponse},
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
        max_ltv: Option<Percent>,
    ) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(self.deps.storage)?;

        let lpp = LppLenderRef::try_new(config.lpp_addr, &self.deps.querier, 0xDEADC0DEDEADC0DE)?;

        let oracle = OracleRef::try_from(config.market_price_oracle, &self.deps.querier)?;

        let resp = lpp.execute(
            Quote::new(
                self.deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.liability,
                config.lease_interest_rate_margin,
                max_ltv,
            ),
            &self.deps.querier,
        )?;

        Ok(resp)
    }
}

pub(super) fn try_setup_dex(
    storage: &mut dyn Storage,
    params: ConnectionParams,
) -> ContractResult<Response> {
    Config::setup_dex(storage, params)?;

    Ok(Response::default())
}

pub(super) fn try_configure(
    storage: &mut dyn Storage,
    lease_interest_rate_margin: Percent,
    liability: Liability,
    lease_interest_payment: InterestPaymentSpec,
) -> ContractResult<Response> {
    Config::update(
        storage,
        lease_interest_rate_margin,
        liability,
        lease_interest_payment,
    )?;

    Ok(Response::default())
}

pub(super) fn try_migrate_leases(
    storage: &mut dyn Storage,
    new_code_id: u64,
) -> ContractResult<Response> {
    Config::update_lease_code(storage, new_code_id)?;

    let mut batch = migrate::migrate_leases(Leases::iter(storage), new_code_id)?;

    update_lpp(storage, new_code_id, &mut batch)?;

    Ok(batch.into())
}

pub(super) fn update_lpp(
    storage: &mut dyn Storage,
    new_code_id: u64,
    batch: &mut Batch,
) -> ContractResult<()> {
    let lpp = Config::load(storage)?.lpp_addr;
    let lpp_update_code = ExecuteMsg::NewLeaseCode {
        lease_code_id: new_code_id.into(),
    };
    batch
        .schedule_execute_wasm_no_reply::<_, Nls>(&lpp, lpp_update_code, None)
        .map_err(Into::into)
}
