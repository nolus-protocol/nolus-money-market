use finance::currency::SymbolOwned;
use lpp::stub::LppRef;
use platform::batch::{Batch, ReplyId};
use sdk::cosmwasm_std::{Addr, Timestamp};
use serde::Deserialize;

use dex::{
    IcaConnector, InRecovery, SwapExactIn as SwapExactInV3, SwapExactInPostRecoverIca,
    SwapExactInPreRecoverIca, TransferOut as TransferOutV3,
};
use oracle::stub::OracleRef;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{DownpaymentCoin, NewLeaseContract, NewLeaseForm},
    contract::{
        cmd::OpenLoanRespResult,
        state::{
            v2::{Account as AccountV2, Migrate},
            Response,
        },
    },
    error::ContractResult,
};

use super::{
    buy_asset::{BuyAsset as BuyAssetV3, DexState},
    open_ica::OpenIcaAccount as OpenIcaAccountV3,
};

type CoinsNb = u8;
pub(in crate::contract) type Transfer = TransferOut;
pub(in crate::contract) type Swap = SwapExactIn;

#[derive(Deserialize)]
pub struct LppLenderRef {
    addr: Addr,
    currency: SymbolOwned,
    #[serde(skip)]
    _open_loan_req_id: ReplyId,
}
impl From<LppLenderRef> for LppRef {
    fn from(value: LppLenderRef) -> Self {
        Self::new(value.addr, value.currency)
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct RequestLoan();
impl Migrate for RequestLoan {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        unreachable!("This state is transient and do not last past a transaction is over")
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl From<OpenIcaAccount> for OpenIcaAccountV3 {
    fn from(value: OpenIcaAccount) -> Self {
        let timealarms = TimeAlarmsRef::unchecked(value.new_lease.form.time_alarms.clone());
        let deps = (value.deps.0.into(), value.deps.1, timealarms);
        Self::new(value.new_lease, value.downpayment, value.loan, deps)
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: AccountV2,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl From<BuyAsset> for BuyAssetV3 {
    fn from(value: BuyAsset) -> Self {
        let timealarms = TimeAlarmsRef::unchecked(value.form.time_alarms.clone());
        let deps = (value.deps.0.into(), value.deps.1, timealarms);
        Self::migrate_to(
            value.form,
            value.dex_account.into(),
            value.downpayment,
            value.loan,
            deps,
        )
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct TransferOut {
    pub spec: BuyAsset,
    pub coin_index: CoinsNb,
    pub last_coin_index: CoinsNb,
}

impl Migrate for TransferOut {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(DexState::from(
            TransferOutV3::migrate_from(self.spec.into(), self.coin_index, self.last_coin_index),
        )))
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct SwapExactIn {
    pub spec: BuyAsset,
}

impl SwapExactIn {
    pub fn into_recovery(self, now: Timestamp) -> ContractResult<(Batch, DexState)> {
        let timealarms = TimeAlarmsRef::unchecked(self.spec.form.time_alarms.clone());
        let pre_recovery = SwapExactInPreRecoverIca::new_migrate(
            IcaConnector::new(InRecovery::new_migrate(self.into(), timealarms.clone())),
            timealarms,
        );
        pre_recovery
            .enter_migrate(now)
            .map(|msgs| (msgs, pre_recovery.into()))
            .map_err(Into::into)
    }

    pub fn into_post_recovery(self) -> DexState {
        let timealarms = TimeAlarmsRef::unchecked(self.spec.form.time_alarms.clone());
        SwapExactInPostRecoverIca::new_migrate(self.into(), timealarms).into()
    }
}

impl Migrate for SwapExactIn {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(DexState::from(Into::<
            SwapExactInV3<BuyAssetV3, DexState>,
        >::into(self))))
    }
}

impl From<SwapExactIn> for SwapExactInV3<BuyAssetV3, DexState> {
    fn from(value: SwapExactIn) -> Self {
        SwapExactInV3::migrate_from(value.spec.into())
    }
}
