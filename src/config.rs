use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use async_trait::async_trait;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;

use rs_utils::config::{Config, Kratos};

pub const CONFIG_FALLBACK: &str = "test/config.toml";

///Represntation of the ports utilized by the web service.
#[derive(Deserialize, Clone, Default, Debug)]
pub struct Ports {
    pub http: String,
    pub http_health: String,
    pub grpc: String,
    pub grpc_health: String,
}

///Represntation of the app eb service config
#[derive(Deserialize, Clone, Default, Debug)]
pub struct Service {
    pub addr: String,
    pub ports: Ports,
}

///Represntation of the tls config.
#[derive(Deserialize, Clone, Default, Debug)]
pub struct Tls {
    pub certificate: String,
    pub key: String,
    pub cert_autority: String,
}


///Representation of this app config.
#[derive(Deserialize, Clone, Default, Debug)]
pub struct PermissionsConfig {
    // pub prefix: String,
    pub service: Service,
    pub kratos: Kratos,
    pub tls: Tls,
    path: Option<PathBuf>,
}

#[async_trait]
impl Config for PermissionsConfig {
    fn set_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }
    ///update the config structure
    async fn update(&mut self) -> Result<()> {
        let path = match self.path {
            Some(ref path) => path as &Path,
            None => bail!("config file path not set"),
        };
        match path.try_exists() {
            Ok(exists) if !exists => bail!("config was not found"),
            Err(e) => bail!(e),
            _ => (),
        }
        let mut config: PermissionsConfig = Figment::new().merge(Toml::file(path)).extract()?;
        config.kratos.update();
        config.set_path(path);
        *self = config;
        Ok(())
    }
}
