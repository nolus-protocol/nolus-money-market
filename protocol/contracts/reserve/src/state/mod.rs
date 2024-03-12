pub use config::Config;

use crate::api::ConfigResponse;

mod config;

impl From<Config> for ConfigResponse {
    fn from(cfg: Config) -> Self {
        Self::new(cfg.lease_code())
    }
}
