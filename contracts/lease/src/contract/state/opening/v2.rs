use cosmwasm_std::Addr;
use finance::currency::SymbolOwned;
use lpp::stub::LppRef;
use platform::batch::ReplyId;
use serde::Deserialize;

use dex::{
    InRecovery, SwapExactIn as SwapExactInV3, SwapExactInPostRecoverIca, SwapExactInRecoverIca,
    TransferOut as TransferOutV3,
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
    fn into_last_version(self) -> Response {
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
    fn into_last_version(self) -> Response {
        Response::no_msgs(DexState::from(TransferOutV3::migrate_from(
            self.spec.into(),
            self.coin_index,
            self.last_coin_index,
        )))
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct SwapExactIn {
    pub spec: BuyAsset,
}

impl SwapExactIn {
    pub fn into_recovery(self) -> DexState {
        let timealarms = TimeAlarmsRef::unchecked(self.spec.form.time_alarms.clone());
        DexState::SwapExactInRecoverIca(SwapExactInRecoverIca::new(InRecovery::new_migrate(
            self.into(),
            timealarms,
        )))
    }

    pub fn into_post_recovery(self) -> DexState {
        let timealarms = TimeAlarmsRef::unchecked(self.spec.form.time_alarms.clone());
        DexState::SwapExactInPostRecoverIca(SwapExactInPostRecoverIca::new_migrate(
            self.into(),
            timealarms,
        ))
    }
}

impl Migrate for SwapExactIn {
    fn into_last_version(self) -> Response {
        Response::no_msgs(DexState::from(
            Into::<SwapExactInV3<BuyAssetV3, DexState>>::into(self),
        ))
    }
}

impl From<SwapExactIn> for SwapExactInV3<BuyAssetV3, DexState> {
    fn from(value: SwapExactIn) -> Self {
        SwapExactInV3::migrate_from(value.spec.into())
    }
}
