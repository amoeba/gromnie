use std::error::Error;
use std::fs;

use gromnie_client::config::{AccountConfig, ConfigLoadError, GromnieConfig, ServerConfig};
use gromnie_runner::{ClientConfig, ClientRunner, LoggingConsumer};
use tracing::info;

pub fn create_example_config(example_config: &str) -> Result<(), Box<dyn Error>> {
    let config_path = GromnieConfig::config_path();

    if config_path.exists() {
        return Err("Config file already exists at {}. Please edit it manually or delete it to create a new one.".into());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&config_path, example_config)?;
    info!("Created example config at {}", config_path.display());
    eprintln!("Config file created at: {}", config_path.display());
    eprintln!("Please edit it with your server and account details, then run gromnie again.");

    Ok(())
}

pub fn load_or_create_config(
    example_config: &str,
) -> Result<Option<GromnieConfig>, Box<dyn Error>> {
    match GromnieConfig::load() {
        Ok(cfg) => {
            info!("Loaded existing config");
            Ok(Some(cfg))
        }
        Err(ConfigLoadError::NotFound) => {
            info!("No config found, creating example config");
            create_example_config(example_config)?;
            Ok(None)
        }
        Err(err) => Err(format!("Failed to load config: {}", err).into()),
    }
}

pub fn resolve_server_and_account<'a>(
    config: &'a GromnieConfig,
    server_name: &str,
    account_name: &str,
) -> Result<(&'a ServerConfig, &'a AccountConfig), Box<dyn Error>> {
    let server = config.servers.get(server_name).ok_or_else(|| {
        let available = config
            .servers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Server '{}' not found. Available servers: {}",
            server_name, available
        )
    })?;

    let account = config.accounts.get(account_name).ok_or_else(|| {
        let available = config
            .accounts
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Account '{}' not found. Available accounts: {}",
            account_name, available
        )
    })?;

    Ok((server, account))
}

pub fn resolve_reconnect(no_reconnect: bool, reconnect: bool, default_reconnect: bool) -> bool {
    if no_reconnect {
        false
    } else if reconnect {
        true
    } else {
        default_reconnect
    }
}

pub fn build_client_config(
    server: &ServerConfig,
    account: &AccountConfig,
    reconnect: bool,
    character_name: Option<String>,
) -> ClientConfig {
    ClientConfig {
        id: 0,
        address: format!("{}:{}", server.host, server.port),
        account_name: account.username.clone(),
        password: account.password.clone(),
        reconnect,
        character_name,
    }
}

pub async fn run_client(
    client_config: ClientConfig,
    config: GromnieConfig,
) -> Result<(), Box<dyn Error>> {
    ClientRunner::builder()
        .with_clients(client_config)
        .with_consumer(LoggingConsumer::from_factory())
        .with_config(config)
        .build()?
        .run()
        .await;

    Ok(())
}
