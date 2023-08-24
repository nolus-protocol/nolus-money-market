use serde::{Deserialize, Serialize};

use currency::{native::Nls, Currency, SymbolOwned};
use finance::{
    coin::{Coin, LpnCoin},
    percent::Percent,
    price::Price,
};
use sdk::{
    cosmwasm_std::{Addr, Uint128, Uint64},
    schemars::{self, JsonSchema},
};

use crate::{borrow::InterestRate, loan::Loan, nlpn::NLpn};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub lpn_ticker: SymbolOwned,
    pub lease_code_admin: Addr,
    pub borrow_rate: InterestRate,
}

#[derive(Serialize, Deserialize)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    NewLeaseCode { lease_code_id: Uint64 },

    OpenLoan { amount: LpnCoin },
    RepayLoan(),

    Deposit(),
    // CW20 interface, withdraw from lender deposit
    Burn { amount: Uint128 },

    DistributeRewards(),
    ClaimRewards { other_recipient: Option<Addr> },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    NewBorrowRate { borrow_rate: InterestRate },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config(),
    Quote {
        amount: LpnCoin,
    },
    Loan {
        lease_addr: Addr,
    },
    // Deposit
    /// CW20 interface, lender deposit balance
    Balance {
        address: Addr,
    },
    LppBalance(),
    Price(),

    Rewards {
        address: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Percent),
    NoLiquidity,
}

pub type LoanResponse<Lpn> = Loan<Lpn>;

pub type QueryLoanResponse<Lpn> = Option<LoanResponse<Lpn>>;

// Deposit query responses

// CW20 interface
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct PriceResponse<LPN>(pub Price<NLpn, LPN>)
where
    LPN: 'static + Currency;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct LppBalanceResponse<LPN>
where
    LPN: Currency,
{
    pub balance: Coin<LPN>,
    pub total_principal_due: Coin<LPN>,
    pub total_interest_due: Coin<LPN>,
    pub balance_nlpn: Coin<NLpn>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct RewardsResponse {
    pub rewards: Coin<Nls>,
}
