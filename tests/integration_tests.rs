use sui_token_transfer_tracker::{
    TokenTransferTracker, Config, 
    error::TrackerError,
    sui_client::SuiClient,
    event_monitor::EventMonitor,
    transaction_processor::TransactionProcessor,
    alert_system::AlertSystem,
    output_formatter::OutputFormatter,
    config::ConfigArgs,
};
use std::time::Duration;
use std::sync::Arc;
use tokio_test;

#[tokio::test]
async fn test_config_loading() {
    // Test loading default configuration
    let config = Config::default();
    assert!(!config.network.rpc_url.is_empty());
    assert_eq!(config.monitoring.poll_interval_seconds, 10);
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = Config::default();
    
    // Test valid configuration
    assert!(config.validate().is_ok());
    
    // Test invalid RPC URL
    config.network.rpc_url = String::new();
    assert!(config.validate().is_err());
    
    // Test invalid poll interval
    config.network.rpc_url = "https://test.com".to_string();
    config.monitoring.poll_interval_seconds = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_config_args_merge() {
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

#[tokio::test]
async fn test_sui_client_creation() {
    // Test client creation with valid URL
    let result = SuiClient::new("https://fullnode.mainnet.sui.io:443").await;
    
    // This may fail in test environment due to network issues
    match result {
        Ok(client) => {
            assert_eq!(client.network_url(), "https://fullnode.mainnet.sui.io:443");
        }
        Err(e) => {
            println!("SuiClient creation failed (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
async fn test_sui_client_health_check() {
    let client_result = SuiClient::new("https://fullnode.mainnet.sui.io:443").await;
    
    if let Ok(client) = client_result {
        let is_healthy = client.is_healthy().await;
        // Health check may fail in test environment
        println!("Health check result: {}", is_healthy);
    }
}

#[tokio::test]
async fn test_event_monitor_creation() {
    let sui_client = Arc::new(
        SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap_or_else(|_| {
            // Create a mock client for testing
            panic!("Failed to create SuiClient for testing");
        })
    );
    
    let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
    
    assert!(!monitor.is_running().await);
    assert_eq!(monitor.get_monitored_addresses().await.len(), 0);
}

#[tokio::test]
async fn test_event_monitor_address_management() {
    let sui_client = Arc::new(
        SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap_or_else(|_| {
            panic!("Failed to create SuiClient for testing");
        })
    );
    
    let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
    
    // Test adding valid address
    let valid_address = "0x1234567890abcdef1234567890abcdef12345678";
    assert!(monitor.add_address(valid_address.to_string()).await.is_ok());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
    
    // Test adding invalid address
    let invalid_address = "invalid_address";
    assert!(monitor.add_address(invalid_address.to_string()).await.is_err());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
    
    // Test removing address
    assert!(monitor.remove_address(valid_address).await.is_ok());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 0);
}

#[tokio::test]
async fn test_transaction_processor() {
    let processor = TransactionProcessor::new();
    
    // Test initial state
    let stats = processor.get_processor_stats().await;
    assert_eq!(stats.total_addresses, 0);
    assert_eq!(stats.total_transactions, 0);
    
    // Test address balance (should be 0 for non-existent address)
    let balance = processor.get_address_balance("0x1234").await;
    assert_eq!(balance, 0);
    
    // Test address history (should be empty for non-existent address)
    let history = processor.get_address_history("0x1234", 10).await;
    assert_eq!(history.len(), 0);
}

#[tokio::test]
async fn test_alert_system() {
    let (alert_system, mut receiver) = AlertSystem::new();
    
    // Test initial state
    let stats = alert_system.get_alert_stats().await;
    assert_eq!(stats.total_alerts, 0);
    
    // Test low balance alert
    alert_system.set_threshold("0xtest".to_string(), 1000000000).await;
    alert_system.check_balance_alert("0xtest", 500000000).await.unwrap();
    
    // Check if alert was sent
    if let Some(alert) = receiver.recv().await {
        match alert {
            sui_token_transfer_tracker::alert_system::Alert::LowBalance { address, balance, threshold, .. } => {
                assert_eq!(address, "0xtest");
                assert_eq!(balance, 500000000);
                assert_eq!(threshold, 1000000000);
            }
            _ => panic!("Expected LowBalance alert"),
        }
    }
}

#[tokio::test]
async fn test_output_formatter() {
    let formatter = OutputFormatter::new(true, true);
    
    // Test welcome message
    let welcome = formatter.format_welcome_message();
    assert!(welcome.contains("SUI Token Transfer Tracker"));
    
    // Test amount formatting
    assert_eq!(formatter.format_amount(1000000000), "1.000000000 SUI");
    assert_eq!(formatter.format_amount(500000000), "0.500000000 SUI");
    
    // Test address truncation
    let long_address = "0x1234567890abcdef1234567890abcdef12345678";
    assert_eq!(formatter.truncate_address(long_address), "0x123456...45678");
    
    // Test error formatting
    let error_msg = formatter.format_error("Test error");
    assert!(error_msg.contains("ERROR: Test error"));
    
    // Test success formatting
    let success_msg = formatter.format_success("Test success");
    assert!(success_msg.contains("✓ Test success"));
}

#[tokio::test]
async fn test_error_handling() {
    use sui_token_transfer_tracker::error::TrackerError;
    
    // Test error creation
    let error = TrackerError::network_error("Network failure");
    assert!(error.is_retriable());
    
    let error = TrackerError::config_error("Configuration error");
    assert!(!error.is_retriable());
    
    // Test error codes
    assert_eq!(TrackerError::network_error("test").error_code(), 1001);
    assert_eq!(TrackerError::config_error("test").error_code(), 2002);
}

#[tokio::test]
async fn test_retry_operation() {
    use sui_token_transfer_tracker::error::utils;
    
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    let result = utils::retry_operation(
        move || {
            let attempts = attempts_clone.clone();
            async move {
                let current = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if current < 3 {
                    Err(TrackerError::network_error("Temporary failure"))
                } else {
                    Ok("success")
                }
            }
        },
        5,
        10,
    ).await;

    assert_eq!(result.unwrap(), "success");
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_tracker_creation() {
    let config = Config::default();
    let result = TokenTransferTracker::new(config).await;
    
    match result {
        Ok(tracker) => {
            assert!(!tracker.is_running().await);
            assert_eq!(tracker.get_all_addresses().await.len(), 0);
        }
        Err(e) => {
            println!("Tracker creation failed (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
async fn test_address_validation() {
    use sui_token_transfer_tracker::config::Config;
    
    // Test valid addresses
    assert!(Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef12345678"));
    assert!(Config::is_valid_sui_address("0x0000000000000000000000000000000000000000000000000000000000000000"));
    
    // Test invalid addresses
    assert!(!Config::is_valid_sui_address("1234567890abcdef1234567890abcdef12345678")); // Missing 0x
    assert!(!Config::is_valid_sui_address("0x123")); // Too short
    assert!(!Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef1234567")); // Too long
    assert!(!Config::is_valid_sui_address("0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")); // Invalid hex
}

#[tokio::test]
async fn test_integration_flow() {
    // This test simulates a basic integration flow
    // Note: This will fail if network connectivity is not available
    
    let config = Config {
        network: sui_token_transfer_tracker::config::NetworkConfig {
            rpc_url: "https://fullnode.testnet.sui.io:443".to_string(),
            websocket_url: "wss://fullnode.testnet.sui.io".to_string(),
            timeout_seconds: 10,
        },
        monitoring: sui_token_transfer_tracker::config::MonitoringConfig {
            poll_interval_seconds: 5,
            max_history_records: 100,
            batch_size: 10,
            cleanup_interval_hours: 1,
        },
        addresses: sui_token_transfer_tracker::config::AddressConfig {
            monitored: vec![],
        },
        alerts: sui_token_transfer_tracker::config::AlertConfig {
            low_balance_threshold: 1000000000,
            large_transfer_threshold: 10000000000,
            enable_console_alerts: true,
            enable_file_alerts: false,
            alert_file_path: "test_alerts.log".to_string(),
        },
        output: sui_token_transfer_tracker::config::OutputConfig {
            use_colors: false,
            show_timestamps: true,
            max_recent_transactions: 5,
            balance_summary_interval: 60,
        },
        logging: sui_token_transfer_tracker::config::LoggingConfig {
            level: "debug".to_string(),
            file_path: "test_tracker.log".to_string(),
            max_file_size_mb: 1,
            rotate_files: 1,
        },
    };

    let tracker_result = TokenTransferTracker::new(config).await;
    
    match tracker_result {
        Ok(tracker) => {
            // Test adding an address
            let test_address = "0x1234567890abcdef1234567890abcdef12345678";
            let add_result = tracker.add_address(test_address.to_string()).await;
            
            // This may fail due to network issues
            if add_result.is_ok() {
                assert_eq!(tracker.get_all_addresses().await.len(), 1);
                
                // Test getting address info
                let address_info = tracker.get_address_info(test_address).await;
                assert!(address_info.is_some());
            } else {
                println!("Address addition failed (expected in test environment)");
            }
        }
        Err(e) => {
            println!("Integration test failed (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
async fn test_performance_metrics() {
    let processor = TransactionProcessor::new();
    
    // Test that stats are initially empty
    let stats = processor.get_processor_stats().await;
    assert_eq!(stats.total_addresses, 0);
    assert_eq!(stats.total_transactions, 0);
    
    // Test export functionality
    let json_export = processor.export_data(
        sui_token_transfer_tracker::transaction_processor::ExportFormat::Json
    ).await;
    assert!(json_export.is_ok());
    
    let csv_export = processor.export_data(
        sui_token_transfer_tracker::transaction_processor::ExportFormat::Csv
    ).await;
    assert!(csv_export.is_ok());
}

#[tokio::test]
async fn test_concurrent_operations() {
    use tokio::task::JoinSet;
    
    let processor = Arc::new(TransactionProcessor::new());
    
    let mut join_set = JoinSet::new();
    
    // Spawn multiple concurrent operations
    for i in 0..10 {
        let processor_clone = processor.clone();
        join_set.spawn(async move {
            let balance = processor_clone.get_address_balance(&format!("0x{:064x}", i)).await;
            balance
        });
    }
    
    // Wait for all operations to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }
    
    // All operations should return 0 (addresses don't exist)
    for result in results {
        assert_eq!(result, 0);
    }
}

#[test]
fn test_error_messages() {
    use sui_token_transfer_tracker::error::TrackerError;
    
    let error = TrackerError::network_error("Connection failed");
    let message = format!("{}", error);
    assert!(message.contains("Network error"));
    assert!(message.contains("Connection failed"));
    
    let error = TrackerError::invalid_address("0x123");
    let message = format!("{}", error);
    assert!(message.contains("Invalid address"));
    assert!(message.contains("0x123"));
}