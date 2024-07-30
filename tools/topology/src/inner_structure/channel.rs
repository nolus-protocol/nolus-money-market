use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Channel {
    pub a: ChannelEndpoint,
    pub b: ChannelEndpoint,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct ChannelEndpoint {
    pub network: Box<str>,
    pub ch: Box<str>,
}
