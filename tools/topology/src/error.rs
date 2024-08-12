use thiserror::Error;

#[derive(Debug, Error)]
pub enum CurrencyDefinitions {
    #[error("Selected DEX network doesn't exist!")]
    NonExistentDexNetwork,
    #[error("Error occurred while processing channel definitions! {0}")]
    ProcessChannels(#[from] ProcessChannels),
    #[error("Host network not connected to DEX network!")]
    HostNotConnectedToDex,
    #[error("Error occurred while resolving currency! {0}")]
    ResolveCurrency(#[from] ResolveCurrency),
}

#[derive(Debug, Error)]
pub enum ProcessChannels {
    #[error("One or more duplicate channel found!")]
    DuplicateChannel,
}

#[derive(Debug, Error)]
pub enum ResolveCurrency {
    #[error(
        "Network defining IBC currency not connected with the source network! \
        Networks: {0:?} & {1:?}"
    )]
    NetworksNotConnected(String, String),
    #[error(
        "Network defining IBC currency points to a previously traversed IBC \
        currency, creating a cycle!"
    )]
    CycleCreated,
    #[error("Defined IBC currency points to non-existent network! Network: {0}")]
    NoSuchNetwork(String),
    #[error(
        "Defined IBC currency points to a non-existent currency on the remote \
        network! Currency: {0}"
    )]
    NoSuchCurrency(String),
}
