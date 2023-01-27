use std::collections::HashSet;

use access_control::SingleUserAccess;

use currency::native::Nls;
use finance::{currency::SymbolOwned, liability::Liability, percent::Percent};
use lease::api::{dex::ConnectionParams, DownpaymentCoin, InterestPaymentSpec};
use lpp::{msg::ExecuteMsg, stub::lender::LppLenderRef};
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, MessageInfo, StdResult, Storage},
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
            )?,
            &self.deps.querier,
        )?;

        Ok(resp)
    }
}

pub struct LeaserAdmin<'a> {
    storage: &'a mut dyn Storage,
}
impl<'a> LeaserAdmin<'a> {
    pub fn new(storage: &'a mut dyn Storage, info: MessageInfo) -> ContractResult<Self> {
        SingleUserAccess::check_owner_access::<ContractError>(storage, &info.sender)?;
        Ok(LeaserAdmin { storage })
    }

    pub fn try_setup_dex(&mut self, params: ConnectionParams) -> ContractResult<Response> {
        Config::setup_dex(self.storage, params)?;

        Ok(Response::default())
    }

    pub fn try_configure(
        &mut self,
        lease_interest_rate_margin: Percent,
        liability: Liability,
        lease_interest_payment: InterestPaymentSpec,
    ) -> ContractResult<Response> {
        Config::update(
            self.storage,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        )?;

        Ok(Response::default())
    }

    pub fn try_migrate_leases(&mut self, new_code_id: u64) -> ContractResult<Response> {
        Config::update_lease_code(self.storage, new_code_id)?;

        let mut batch = migrate::migrate_leases(Leases::iter(self.storage), new_code_id)?;

        self.update_lpp(new_code_id, &mut batch)?;

        Ok(batch.into())
    }

    fn update_lpp(&mut self, new_code_id: u64, batch: &mut Batch) -> ContractResult<()> {
        let lpp = Config::load(self.storage)?.lpp_addr;
        let lpp_update_code = ExecuteMsg::NewLeaseCode {
            lease_code_id: new_code_id.into(),
        };
        batch
            .schedule_execute_wasm_no_reply::<_, Nls>(&lpp, lpp_update_code, None)
            .map_err(Into::into)
    }
}
