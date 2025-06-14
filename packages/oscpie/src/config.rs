use crate::prelude::*;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::versioned::{CompositMigrator, Versioned};

mod v1;

pub mod types {
    pub use super::v1::*;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "config_version", content = "config")]
pub enum ConfigFile {
    V1(v1::Config),
}

impl Versioned<u32> for ConfigFile {
    fn version(&self) -> u32 {
        match self {
            ConfigFile::V1(_) => 1,
        }
    }
}

pub type Config = v1::Config;

fn migrator() -> CompositMigrator<ConfigFile, u32> {
    CompositMigrator::new()
}

pub fn read(config_file: ConfigFile) -> Result<Config> {
    let migrator = migrator();

    let migrated = migrator.migrate(config_file, 1);

    let Ok(ConfigFile::V1(config)) = migrated else {
        return Err(anyhow!("Failed to migrate config"));
    };

    Ok(config)
}

pub fn load(path: &str) -> Result<Config> {
    let file = std::fs::File::open(path).map_err(|e| anyhow!(e.to_string()))?;
    let config_file: ConfigFile =
        serde_json::from_reader(file).map_err(|e| anyhow!(e.to_string()))?;

    let config = read(config_file)?;

    // TODO: Migrate and save to new version if needed

    Ok(config)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_config() {
        let config = load("test_files/config/config.json");
        assert!(config.is_ok());
    }
}
