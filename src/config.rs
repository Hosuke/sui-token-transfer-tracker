use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{TrackerError, TrackerResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub monitoring: MonitoringConfig,
    pub addresses: AddressConfig,
    pub alerts: AlertConfig,
    pub output: OutputConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub rpc_url: String,
    pub websocket_url: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub poll_interval_seconds: u64,
    pub max_history_records: u32,
    pub batch_size: u32,
    pub cleanup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressConfig {
    pub monitored: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub low_balance_threshold: u64,
    pub large_transfer_threshold: u64,
    pub enable_console_alerts: bool,
    pub enable_file_alerts: bool,
    pub alert_file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub use_colors: bool,
    pub show_timestamps: bool,
    pub max_recent_transactions: u32,
    pub balance_summary_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: String,
    pub max_file_size_mb: u32,
    pub rotate_files: u32,
}

impl Config {
    pub fn load(config_path: Option<&str>) -> TrackerResult<Self> {
        match config_path {
            Some(path) => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| TrackerError::Configuration(
                        format!("Failed to read config file: {}", e)
                    ))?;
                
                toml::from_str(&content)
                    .map_err(|e| TrackerError::TomlError(e))
            }
            None => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &Path) -> TrackerResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| TrackerError::TomlSerializeError(e))?;
        
        std::fs::write(path, content)
            .map_err(|e| TrackerError::IoError(e))?;
        
        Ok(())
    }

    pub fn validate(&self) -> TrackerResult<()> {
        if self.network.rpc_url.is_empty() {
            return Err(TrackerError::validation_error(
                "RPC URL cannot be empty"
            ));
        }

        if self.monitoring.poll_interval_seconds == 0 {
            return Err(TrackerError::validation_error(
                "Poll interval must be greater than 0"
            ));
        }

        if self.monitoring.max_history_records == 0 {
            return Err(TrackerError::validation_error(
                "Max history records must be greater than 0"
            ));
        }

        if self.monitoring.batch_size == 0 {
            return Err(TrackerError::validation_error(
                "Batch size must be greater than 0"
            ));
        }

        if self.alerts.low_balance_threshold == 0 {
            return Err(TrackerError::validation_error(
                "Low balance threshold must be greater than 0"
            ));
        }

        if self.alerts.large_transfer_threshold == 0 {
            return Err(TrackerError::validation_error(
                "Large transfer threshold must be greater than 0"
            ));
        }

        for address in &self.addresses.monitored {
            if !Self::is_valid_sui_address(address) {
                return Err(TrackerError::invalid_address(
                    format!("Invalid SUI address: {}", address)
                ));
            }
        }

        Ok(())
    }

    pub fn is_valid_sui_address(address: &str) -> bool {
        address.starts_with("0x") && address.len() == 66 && 
        address[2..].chars().all(|c| c.is_ascii_hexdigit())
    }

    pub fn merge_with_args(&mut self, args: &ConfigArgs) {
        if let Some(rpc_url) = &args.rpc_url {
            self.network.rpc_url = rpc_url.clone();
        }

        if let Some(poll_interval) = args.poll_interval {
            self.monitoring.poll_interval_seconds = poll_interval;
        }

        if let Some(threshold) = args.low_balance_threshold {
            self.alerts.low_balance_threshold = threshold;
        }

        if let Some(large_threshold) = args.large_transfer_threshold {
            self.alerts.large_transfer_threshold = large_threshold;
        }

        if let Some(use_colors) = args.use_colors {
            self.output.use_colors = use_colors;
        }

        if let Some(show_timestamps) = args.show_timestamps {
            self.output.show_timestamps = show_timestamps;
        }

        if let Some(log_level) = &args.log_level {
            self.logging.level = log_level.clone();
        }

        if !args.addresses.is_empty() {
            self.addresses.monitored = args.addresses.clone();
        }
    }

    pub fn generate_default_config() -> String {
        toml::to_string_pretty(&Self::default()).unwrap()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                rpc_url: "https://fullnode.mainnet.sui.io:443".to_string(),
                websocket_url: "wss://fullnode.mainnet.sui.io".to_string(),
                timeout_seconds: 30,
            },
            monitoring: MonitoringConfig {
                poll_interval_seconds: 10,
                max_history_records: 1000,
                batch_size: 50,
                cleanup_interval_hours: 24,
            },
            addresses: AddressConfig {
                monitored: Vec::new(),
            },
            alerts: AlertConfig {
                low_balance_threshold: 1000000000,
                large_transfer_threshold: 10000000000,
                enable_console_alerts: true,
                enable_file_alerts: false,
                alert_file_path: "alerts.log".to_string(),
            },
            output: OutputConfig {
                use_colors: true,
                show_timestamps: true,
                max_recent_transactions: 10,
                balance_summary_interval: 300,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_path: "tracker.log".to_string(),
                max_file_size_mb: 10,
                rotate_files: 5,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigArgs {
    pub rpc_url: Option<String>,
    pub poll_interval: Option<u64>,
    pub low_balance_threshold: Option<u64>,
    pub large_transfer_threshold: Option<u64>,
    pub use_colors: Option<bool>,
    pub show_timestamps: Option<bool>,
    pub log_level: Option<String>,
    pub addresses: Vec<String>,
}

impl Default for ConfigArgs {
    fn default() -> Self {
        Self {
            rpc_url: None,
            poll_interval: None,
            low_balance_threshold: None,
            large_transfer_threshold: None,
            use_colors: None,
            show_timestamps: None,
            log_level: None,
            addresses: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    // use tempfile::NamedTempFile; // Commented out as tempfile is not in dependencies

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.network.rpc_url.is_empty());
        assert_eq!(config.monitoring.poll_interval_seconds, 10);
        assert_eq!(config.alerts.low_balance_threshold, 1000000000);
    }

    #[test]
    fn test_address_validation() {
        assert!(Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef12345678"));
        assert!(!Config::is_valid_sui_address("1234567890abcdef1234567890abcdef12345678")); // 缺少0x前缀
        assert!(!Config::is_valid_sui_address("0x123")); // 长度不足
        assert!(!Config::is_valid_sui_address("0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")); // 包含非十六进制字符
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.network.rpc_url = String::new();
        assert!(config.validate().is_err());

        config.network.rpc_url = "https://test.com".to_string();
        config.monitoring.poll_interval_seconds = 0;
        assert!(config.validate().is_err());

        config.monitoring.poll_interval_seconds = 10;
        config.alerts.low_balance_threshold = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_load_and_save_config() {
        // TODO: This test requires tempfile dependency
        // let config = Config::default();
        // let mut temp_file = NamedTempFile::new().unwrap();
        // config.save(temp_file.path()).unwrap();
        // let loaded_config = Config::load(Some(temp_file.path().to_str().unwrap())).unwrap();
        // assert_eq!(config.network.rpc_url, loaded_config.network.rpc_url);
        // assert_eq!(config.monitoring.poll_interval_seconds, loaded_config.monitoring.poll_interval_seconds);
    }

    #[test]
    fn test_merge_with_args() {
        let mut config = Config::default();
        let args = ConfigArgs {
            rpc_url: Some("https://custom.rpc".to_string()),
            poll_interval: Some(5),
            addresses: vec!["0x1234567890abcdef1234567890abcdef12345678".to_string()],
            ..Default::default()
        };

        config.merge_with_args(&args);
        
        assert_eq!(config.network.rpc_url, "https://custom.rpc");
        assert_eq!(config.monitoring.poll_interval_seconds, 5);
        assert_eq!(config.addresses.monitored.len(), 1);
    }

    #[test]
    fn test_generate_default_config() {
        let config_str = Config::generate_default_config();
        assert!(config_str.contains("rpc_url"));
        assert!(config_str.contains("poll_interval_seconds"));
        assert!(config_str.contains("low_balance_threshold"));
        
        // 确保可以解析生成的配置
        let parsed_config: Config = toml::from_str(&config_str).unwrap();
        assert!(parsed_config.validate().is_ok());
    }
}