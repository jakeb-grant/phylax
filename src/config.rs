use std::path::Path;

use eyre::Result;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    helper_path: String,
    socket_path: String,
}

impl SystemConfig {
    pub fn from_file() -> Result<Self> {
        let mut fig = Figment::new();
        // Prioritize configuration in local, as semantically that is the users config
        if Path::new("/usr/local/etc/phylax/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/usr/local/etc/phylax/config.toml"));
            tracing::info!("using configuration file found at /usr/local/etc/phylax/config.toml");
        // Try the configuration location of the distro
        } else if Path::new("/etc/phylax/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/etc/phylax/config.toml"));
            tracing::info!("using configuration file found at /etc/phylax/config.toml");
        // Fall back to default
        } else {
            fig = fig.merge(Serialized::defaults(Self::default()));
            tracing::info!("no configuration file found, using default configuration instead");
        }
        Ok(fig.extract()?)
    }

    pub fn get_helper_path(&self) -> &str {
        &self.helper_path
    }

    pub fn get_socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            helper_path: env!("POLKIT_AGENT_HELPER_PATH").into(),
            socket_path: "/run/polkit/agent-helper.socket".into(),
        }
    }
}
