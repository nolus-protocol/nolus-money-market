use currencies::{Lpn, Lpns};

pub(crate) type LpnCurrencies = Lpns;
pub(crate) type LpnCurrency = Lpn;

pub(crate) type OracleRef = oracle_platform::OracleRef<LpnCurrency>;
