use std::collections::HashSet;

use finance::{currency::SymbolOwned, liability::Liability, percent::Percent};
use lease::api::{dex::ConnectionParams, DownpaymentCoin, InterestPaymentSpec};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::access_control::SingleUserAccess;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, MessageInfo, StdResult, Storage},
};

use crate::{
    cmd::Quote,
    error::{ContractError, ContractResult},
    msg::{ConfigResponse, QuoteResponse},
    state::{
        config::Config,
        leaser::Loans,
    },
};

pub struct Leaser {}

impl Leaser {
    pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
        let config = Config::load(deps.storage)?;
        Ok(ConfigResponse { config })
    }

    pub fn query_loans(deps: Deps, owner: Addr) -> StdResult<HashSet<Addr>> {
        Loans::get(deps.storage, owner)
    }

    pub fn query_quote(
        deps: Deps,
        downpayment: DownpaymentCoin,
        lease_asset: SymbolOwned,
    ) -> Result<QuoteResponse, ContractError> {
        let config = Config::load(deps.storage)?;

        let lpp = LppLenderRef::try_new(config.lpp_addr, &deps.querier, 0xDEADC0DEDEADC0DE)?;

        let oracle = OracleRef::try_from(config.market_price_oracle, &deps.querier)?;

        let resp = lpp.execute(
            Quote::new(
                deps.querier,
                downpayment,
                lease_asset,
                oracle,
                config.liability,
                config.lease_interest_rate_margin,
            )?,
            &deps.querier,
        )?;

        Ok(resp)
    }

    pub fn try_setup_dex(
        storage: &mut dyn Storage,
        info: MessageInfo,
        params: ConnectionParams,
    ) -> ContractResult<Response> {
        SingleUserAccess::load(storage, crate::access_control::OWNER_NAMESPACE)?
            .check_access(&info.sender)?;

        Config::setup_dex(storage, params)?;

        Ok(Response::default())
    }

    pub fn try_configure(
        storage: &mut dyn Storage,
        info: MessageInfo,
        lease_interest_rate_margin: Percent,
        liability: Liability,
        lease_interest_payment: InterestPaymentSpec,
    ) -> ContractResult<Response> {
        SingleUserAccess::load(storage, crate::access_control::OWNER_NAMESPACE)?
            .check_access(&info.sender)?;

        Config::update(
            storage,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        )?;

        Ok(Response::default())
    }
}
