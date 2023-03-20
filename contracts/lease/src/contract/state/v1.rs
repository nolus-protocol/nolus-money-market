use crate::{
    api::{
        dex::ConnectionParams, DownpaymentCoin, LoanForm, LpnCoin,
        NewLeaseContract as NewLeaseContractV2, NewLeaseForm as NewLeaseFormV2, PaymentCoin,
    },
    contract::{cmd::OpenLoanRespResult, dex::Account, Lease},
};
use cosmwasm_std::{Addr, Timestamp};
use enum_dispatch::enum_dispatch;
use finance::{currency::SymbolOwned, liability::Liability};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use serde::{Deserialize, Serialize, Serializer};

use super::{
    closed::Closed,
    ica_connector::{self, IcaConnector},
    ica_recover::{self, InRecovery},
    opened::{
        self,
        repay::{
            buy_lpn::BuyLpn, transfer_in_finish::TransferInFinish as PaymentTransferInFinishV2,
        },
    },
    opening::{
        buy_asset::{BuyAsset as BuyAssetV2, Swap, Transfer},
        open_ica::OpenIcaAccount as OpenIcaAccountV2,
        request_loan::RequestLoan as RequestLoanV2,
    },
    paid::{self, transfer_in_finish::TransferInFinish as PaidTransferInFinishV2},
    State as StateV2,
};

type OpenIcaAccount = ica_connector::IcaConnector<OpenIcaAccountV1>;
type OpeningTransferOut = TransferOutV1;
type BuyAsset = BuyAssetV1;
type BuyAssetRecoverIca = ica_connector::IcaConnector<ica_recover::InRecovery<BuyAssetV1>>;
type OpenedActive = opened::active::Active;
type RepaymentTransferOut = opened::repay::transfer_out::TransferOut;
type BuyLpnRecoverIca = ica_connector::IcaConnector<ica_recover::InRecovery<BuyLpn>>;
type RepaymentTransferInInit = opened::repay::transfer_in_init::TransferInInit;
type RepaymentTransferInInitRecoverIca =
    ica_connector::IcaConnector<ica_recover::InRecovery<RepaymentTransferInInit>>;
type RepaymentTransferInFinish = PaymentTransferInFinishV1;
type PaidActive = paid::Active;
type ClosingTransferInInit = paid::transfer_in_init::TransferInInit;
type ClosingTransferInInitRecoverIca =
    ica_connector::IcaConnector<ica_recover::InRecovery<ClosingTransferInInit>>;
type ClosingTransferInFinish = PaidTransferInFinishV1;

//2023-03-16T22:53:20
const POINT_IN_THE_PAST: Timestamp = Timestamp::from_seconds(1679000000);

#[enum_dispatch]
pub(super) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self) -> StateV2;
}

#[enum_dispatch(Migrate)]
#[derive(Deserialize)]
pub(super) enum StateV1 {
    RequestLoan,
    OpenIcaAccount,
    OpeningTransferOut,
    BuyAsset,
    BuyAssetRecoverIca,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    BuyLpnRecoverIca,
    RepaymentTransferInInit,
    RepaymentTransferInInitRecoverIca,
    RepaymentTransferInFinish,
    PaidActive,
    ClosingTransferInInit,
    ClosingTransferInInitRecoverIca,
    ClosingTransferInFinish,
    Closed,
}

#[derive(Deserialize)]
pub(super) struct RequestLoan {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    deps: (LppLenderRef, OracleRef),
}
impl Migrate for RequestLoan {
    fn into_last_version(self) -> StateV2 {
        RequestLoanV2 {
            new_lease: self.new_lease.into(),
            downpayment: self.downpayment,
            deps: self.deps,
        }
        .into()
    }
}

#[derive(Deserialize)]
pub(super) struct OpenIcaAccountV1 {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}
impl From<OpenIcaAccountV1> for OpenIcaAccountV2 {
    fn from(value: OpenIcaAccountV1) -> Self {
        Self {
            new_lease: value.new_lease.into(),
            downpayment: value.downpayment,
            loan: value.loan,
            deps: value.deps,
        }
    }
}
impl Migrate for IcaConnector<OpenIcaAccountV1> {
    fn into_last_version(self) -> StateV2 {
        IcaConnector::<OpenIcaAccountV2> {
            connectee: self.connectee.into(),
        }
        .into()
    }
}

type TransfersNb = u8;
#[derive(Deserialize)]
pub(super) struct TransferOutV1 {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
    nb_completed: TransfersNb,
}
impl Migrate for TransferOutV1 {
    fn into_last_version(self) -> StateV2 {
        let spec = BuyAssetV2::new(
            self.form.into(),
            self.dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        );
        Transfer::new_migrate_v1(spec, self.nb_completed).into()
    }
}

#[derive(Deserialize)]
pub(super) struct BuyAssetV1 {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}
impl From<BuyAssetV1> for BuyAssetV2 {
    fn from(value: BuyAssetV1) -> Self {
        Self::new(
            value.form.into(),
            value.dex_account,
            value.downpayment,
            value.loan,
            value.deps,
        )
    }
}
impl Migrate for BuyAssetV1 {
    fn into_last_version(self) -> StateV2 {
        Swap::new(self.into()).into()
    }
}
impl Migrate for BuyAssetRecoverIca {
    fn into_last_version(self) -> StateV2 {
        let swap = Swap::new(self.connectee.into_state().into());
        IcaConnector::new(InRecovery::new(swap)).into()
    }
}

impl Migrate for OpenedActive {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for RepaymentTransferOut {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for BuyLpn {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for BuyLpnRecoverIca {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for RepaymentTransferInInit {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for RepaymentTransferInInitRecoverIca {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}

#[derive(Deserialize)]
pub(super) struct PaymentTransferInFinishV1 {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}
impl Migrate for PaymentTransferInFinishV1 {
    fn into_last_version(self) -> StateV2 {
        PaymentTransferInFinishV2::new(
            self.lease,
            self.payment,
            self.payment_lpn,
            POINT_IN_THE_PAST,
        )
        .into()
    }
}
impl Migrate for PaidActive {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for ClosingTransferInInit {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}
impl Migrate for ClosingTransferInInitRecoverIca {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}

#[derive(Deserialize)]
pub(super) struct PaidTransferInFinishV1 {
    lease: Lease,
}
impl Migrate for PaidTransferInFinishV1 {
    fn into_last_version(self) -> StateV2 {
        PaidTransferInFinishV2::new(self.lease, POINT_IN_THE_PAST).into()
    }
}

impl Migrate for Closed {
    fn into_last_version(self) -> StateV2 {
        self.into()
    }
}

#[derive(Deserialize)]
struct NewLeaseContract {
    /// An application form for opening a new lease
    form: NewLeaseForm,
    /// Connection parameters of a Dex capable to perform currency swaps
    dex: ConnectionParams,
}
impl From<NewLeaseContract> for NewLeaseContractV2 {
    fn from(value: NewLeaseContract) -> Self {
        Self {
            form: value.form.into(),
            dex: value.dex,
        }
    }
}

#[derive(Deserialize)]
struct NewLeaseForm {
    /// The customer who wants to open a lease.
    customer: Addr,
    /// Ticker of the currency this lease will be about.
    currency: SymbolOwned,
    /// Liability parameters
    liability: Liability,
    /// Loan parameters
    loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    time_alarms: Addr,
    /// The oracle contract that sends market price alerts to the lease
    market_price_oracle: Addr,
}
impl From<NewLeaseForm> for NewLeaseFormV2 {
    fn from(value: NewLeaseForm) -> Self {
        Self {
            customer: value.customer,
            currency: value.currency,
            liability: value.liability,
            loan: value.loan,
            time_alarms: value.time_alarms,
            market_price_oracle: value.market_price_oracle,
            max_ltv: None,
        }
    }
}

impl Serialize for StateV1 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
