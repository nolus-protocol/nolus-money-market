pub(crate) use currencies::{
    LeaseGroup as LeaseCurrencies, Lpn as LpnCurrency, Lpns as LpnCurrencies,
    PaymentGroup as PaymentCurrencies,
};

pub(crate) type OracleRef = oracle_platform::OracleRef<LpnCurrency, LpnCurrencies>;
