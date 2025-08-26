use serde::{Deserialize, Serialize};

use currency::{Group, platform::Nls};
use finance::{
    coin::{Coin, CoinDTO},
    percent::{Percent, bound::BoundToHundredPercent},
    price::Price,
};
use lpp_platform::NLpn;
use platform::contract::Code;
use sdk::cosmwasm_std::{Addr, Uint64};

use crate::{borrow::InterestRate, config::Config, loan::Loan};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// The actor who has rights to change the Lease code on code migrations,
    /// and close all deposits.
    /// In the current protocol architecture, it is the leaser contract
    pub protocol_admin: Addr,
    // Since this is an external system API we should not use [Code].
    pub lease_code: Uint64,
    pub borrow_rate: InterestRate,
    pub min_utilization: BoundToHundredPercent,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum ExecuteMsg<Lpns>
where
    Lpns: Group,
{
    NewLeaseCode {
        // This is an internal system API and we use [Code]
        lease_code: Code,
    },

    OpenLoan {
        amount: CoinDTO<Lpns>,
    },
    RepayLoan(),

    Deposit(),
    // CW20 interface, withdraw from lender deposit
    Burn {
        amount: Coin<NLpn>,
    },
    /// Close all customer deposits as if their owners have burnt the full deposit amounts
    ///
    /// Pre: The caller is the Lease Code Admin.
    /// Pre: There are no outstanding loans.
    CloseAllDeposits(),

    /// Implementation of lpp_platform::msg::ExecuteMsg::DistributeRewards
    DistributeRewards(),
    ClaimRewards {
        other_recipient: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {
    NewBorrowRate {
        borrow_rate: InterestRate,
    },
    MinUtilization {
        min_utilization: BoundToHundredPercent,
    },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum QueryMsg<Lpns>
where
    Lpns: Group,
{
    /// Return the configuration in [ConfigResponse]
    Config(),
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
    /// Report the Lpn currency as [CurrencyDTO<Lpns>]
    Lpn(),
    Quote {
        amount: CoinDTO<Lpns>,
    },
    Loan {
        lease_addr: Addr,
    },
    // Deposit
    /// CW20 interface, lender deposit balance
    Balance {
        address: Addr,
    },

    /// Return the pool's total balance in Lpn [LppBalanceResponse]
    LppBalance(),

    /// Implementation of [lpp_platform::msg::QueryMsg::StableBalance]
    StableBalance {
        oracle_addr: Addr,
    },

    Price(),
    DepositCapacity(),

    Rewards {
        address: Addr,
    },
}

pub type ConfigResponse = Config;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Percent),
    NoLiquidity,
}

pub type LoanResponse<Lpn> = Loan<Lpn>;

pub type QueryLoanResponse<Lpn> = Option<LoanResponse<Lpn>>;

// Deposit query responses

// CW20 interface
/// A response to `QueryMsg::Balance`\
/// Returns the lender's total balance in NLpn
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BalanceResponse {
    #[serde(flatten)]
    pub balance: Coin<NLpn>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub struct LppBalanceResponse<Lpns>
where
    Lpns: Group,
{
    pub balance: CoinDTO<Lpns>,
    pub total_principal_due: CoinDTO<Lpns>,
    pub total_interest_due: CoinDTO<Lpns>,
    pub balance_nlpn: Coin<NLpn>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PriceResponse<Lpn>(pub Price<NLpn, Lpn>)
where
    Lpn: 'static;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RewardsResponse {
    pub rewards: Coin<Nls>,
}

#[cfg(test)]
mod test {
    use currencies::Lpns;
    use finance::coin::Coin;
    use platform::tests::{self as platform_tests};

    use super::QueryMsg;
    use crate::msg::BalanceResponse;

    #[test]
    fn release() {
        assert_eq!(
            Ok(QueryMsg::<Lpns>::ProtocolPackageRelease {}),
            platform_tests::ser_de(&versioning::query::ProtocolPackage::Release {}),
        );

        platform_tests::ser_de::<_, QueryMsg<Lpns>>(
            &versioning::query::PlatformPackage::Release {},
        )
        .unwrap_err();
    }

    #[test]
    fn balance_response_string() {
        let response = BalanceResponse {
            balance: Coin::new(1_000),
        };

        platform_tests::assert_ser_string(&response, "{\"amount\":\"1000\"}");
    }
}
