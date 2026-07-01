use crate::api::ConfigResponse;

pub use self::config::Config;

mod config;

impl From<Config> for ConfigResponse {
    fn from(cfg: Config) -> Self {
        Self {
            owner: cfg.owner().clone(),
        }
    }
}
