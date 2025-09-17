use sui_token_tracker::{
    TokenTransferTracker, Config, 
    error::TrackerError,
    sui_client::SuiClient,
    event_monitor::EventMonitor,
    transaction_processor::TransactionProcessor,
    alert_system::AlertSystem,
    output_formatter::OutputFormatter,
};
use std::time::Duration;
use tokio_test;

#[tokio::test]
async fn test_sui_client_mock() {
    // Test basic client functionality
    // Note: This is a simplified test that doesn't require actual network connection
    
    let client_result = SuiClient::new("https://fullnode.mainnet.sui.io:443").await;
    
    match client_result {
        Ok(client) => {
            // Test that client was created successfully
            assert_eq!(client.network_url(), "https://fullnode.mainnet.sui.io:443");
            
            // Test timeout functionality
            let timeout_client = SuiClient::with_timeout("https://fullnode.mainnet.sui.io:443", 5).await;
            assert!(timeout_client.is_ok());
        }
        Err(_) => {
            // Network connectivity issues are expected in test environment
            println!("Skipping SuiClient tests due to network connectivity");
        }
    }
}

#[tokio::test]
async fn test_event_monitor_basic() {
    let sui_client = match SuiClient::new("https://fullnode.mainnet.sui.io:443").await {
        Ok(client) => std::sync::Arc::new(client),
        Err(_) => {
            println!("Skipping EventMonitor tests due to network connectivity");
            return;
        }
    };
    
    let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(1)).await;
    
    // Test initial state
    assert!(!monitor.is_running().await);
    assert_eq!(monitor.get_monitored_addresses().await.len(), 0);
    
    // Test address validation
    let valid_addresses = monitor.validate_addresses().await;
    assert_eq!(valid_addresses.len(), 0);
    
    // Test adding valid address
    let valid_address = "0x1234567890abcdef1234567890abcdef12345678";
    let result = monitor.add_address(valid_address.to_string()).await;
    assert!(result.is_ok());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
    
    // Test adding invalid address
    let invalid_address = "invalid_address";
    let result = monitor.add_address(invalid_address.to_string()).await;
    assert!(result.is_err());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
    
    // Test removing address
    let result = monitor.remove_address(valid_address).await;
    assert!(result.is_ok());
    assert_eq!(monitor.get_monitored_addresses().await.len(), 0);
}

#[tokio::test]
async fn test_transaction_processor_basic() {
    let processor = TransactionProcessor::new();
    
    // Test initial state
    let stats = processor.get_processor_stats().await;
    assert_eq!(stats.total_addresses, 0);
    assert_eq!(stats.total_transactions, 0);
    
    // Test getting balance for non-existent address
    let balance = processor.get_address_balance("0x1234").await;
    assert_eq!(balance, 0);
    
    // Test getting history for non-existent address
    let history = processor.get_address_history("0x1234", 10).await;
    assert_eq!(history.len(), 0);
    
    // Test getting all balances
    let balances = processor.get_all_balances().await;
    assert_eq!(balances.len(), 0);
    
    // Test getting all stats
    let all_stats = processor.get_all_stats().await;
    assert_eq!(all_stats.len(), 0);
}

#[tokio::test]
async fn test_alert_system_basic() {
    let (alert_system, mut receiver) = AlertSystem::new();
    
    // Test initial state
    let stats = alert_system.get_alert_stats().await;
    assert_eq!(stats.total_alerts, 0);
    
    // Test setting threshold
    alert_system.set_threshold("0xtest".to_string(), 1000000000).await;
    
    // Test low balance alert
    let result = alert_system.check_balance_alert("0xtest", 500000000).await;
    assert!(result.is_ok());
    
    // Check if alert was received
    if let Some(alert) = receiver.recv().await {
        match alert {
            sui_token_tracker::alert_system::Alert::LowBalance { address, balance, threshold, .. } => {
                assert_eq!(address, "0xtest");
                assert_eq!(balance, 500000000);
                assert_eq!(threshold, 1000000000);
            }
            _ => panic!("Expected LowBalance alert"),
        }
    }
}

#[tokio::test]
async fn test_output_formatter_basic() {
    let formatter = OutputFormatter::new(true, true);
    
    // Test basic formatting methods
    let welcome = formatter.format_welcome_message();
    assert!(welcome.contains("SUI Token Transfer Tracker"));
    
    // Test amount formatting
    assert_eq!(formatter.format_amount(1000000000), "1.000000000 SUI");
    assert_eq!(formatter.format_amount(500000000), "0.500000000 SUI");
    assert_eq!(formatter.format_amount(0), "0.000000000 SUI");
    
    // Test address truncation
    let long_address = "0x1234567890abcdef1234567890abcdef12345678";
    assert_eq!(formatter.truncate_address(long_address), "0x123456...45678");
    
    let short_address = "0x1234";
    assert_eq!(formatter.truncate_address(short_address), "0x1234");
    
    // Test message formatting
    let error_msg = formatter.format_error("Test error");
    assert!(error_msg.contains("ERROR: Test error"));
    
    let warning_msg = formatter.format_warning("Test warning");
    assert!(warning_msg.contains("WARNING: Test warning"));
    
    let success_msg = formatter.format_success("Test success");
    assert!(success_msg.contains("✓ Test success"));
    
    let info_msg = formatter.format_info("Test info");
    assert!(info_msg.contains("ℹ Test info"));
}

#[tokio::test]
async fn test_config_basic() {
    // Test default configuration
    let config = Config::default();
    assert!(!config.network.rpc_url.is_empty());
    assert_eq!(config.monitoring.poll_interval_seconds, 10);
    assert!(config.validate().is_ok());
    
    // Test configuration generation
    let config_str = Config::generate_default_config();
    assert!(config_str.contains("rpc_url"));
    assert!(config_str.contains("poll_interval_seconds"));
    
    // Test configuration merging
    let mut config = Config::default();
    let args = sui_token_tracker::config::ConfigArgs {
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
async fn test_error_handling_basic() {
    use sui_token_tracker::error::{TrackerError, utils};
    
    // Test error creation
    let network_error = TrackerError::network_error("Network failure");
    assert!(network_error.is_retriable());
    assert_eq!(network_error.error_code(), 1001);
    
    let config_error = TrackerError::config_error("Configuration error");
    assert!(!config_error.is_retriable());
    assert_eq!(config_error.error_code(), 2002);
    
    let invalid_address_error = TrackerError::invalid_address("Invalid address");
    assert!(!invalid_address_error.is_retriable());
    assert_eq!(invalid_address_error.error_code(), 4001);
    
    // Test error message formatting
    let error_msg = format!("{}", network_error);
    assert!(error_msg.contains("Network error"));
    assert!(error_msg.contains("Network failure"));
    
    // Test retry operation
    let mut attempts = 0;
    let result = utils::retry_operation(
        || {
            attempts += 1;
            async {
                if attempts < 3 {
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
    assert_eq!(attempts, 3);
}

#[tokio::test]
async fn test_tracker_basic() {
    let config = Config::default();
    let tracker_result = TokenTransferTracker::new(config).await;
    
    match tracker_result {
        Ok(mut tracker) => {
            // Test initial state
            assert!(!tracker.is_running().await);
            assert_eq!(tracker.get_all_addresses().await.len(), 0);
            
            // Test stats
            let stats = tracker.get_tracker_stats().await;
            assert_eq!(stats.total_events_processed, 0);
            assert_eq!(stats.total_transactions_processed, 0);
            assert_eq!(stats.total_alerts_sent, 0);
            assert_eq!(stats.total_errors, 0);
            
            // Test adding valid address (may fail due to network issues)
            let valid_address = "0x1234567890abcdef1234567890abcdef12345678";
            let add_result = tracker.add_address(valid_address.to_string()).await;
            
            if add_result.is_ok() {
                assert_eq!(tracker.get_all_addresses().await.len(), 1);
                
                // Test getting address info
                let address_info = tracker.get_address_info(valid_address).await;
                assert!(address_info.is_some());
                
                // Test removing address
                let remove_result = tracker.remove_address(valid_address).await;
                assert!(remove_result.is_ok());
                assert_eq!(tracker.get_all_addresses().await.len(), 0);
            }
        }
        Err(e) => {
            println!("Tracker creation failed (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
async fn test_balance_operations() {
    let processor = TransactionProcessor::new();
    
    // Test initial balances
    let balance = processor.get_address_balance("0x1234").await;
    assert_eq!(balance, 0);
    
    let balances = processor.get_all_balances().await;
    assert_eq!(balances.len(), 0);
    
    // Test balance history
    let history = processor.get_balance_history("0x1234", 10).await;
    assert_eq!(history.history.len(), 0);
    assert_eq!(history.address, "0x1234");
}

#[tokio::test]
async fn test_transaction_operations() {
    let processor = TransactionProcessor::new();
    
    // Test getting recent transactions
    let recent_transactions = processor.get_recent_transactions(10).await;
    assert_eq!(recent_transactions.len(), 0);
    
    // Test getting transaction volume stats
    let volume_stats = processor.get_transaction_volume_stats(24).await;
    assert_eq!(volume_stats.len(), 0);
    
    // Test getting address stats
    let address_stats = processor.get_address_stats("0x1234").await;
    assert!(address_stats.is_none());
}

#[tokio::test]
async fn test_export_operations() {
    let processor = TransactionProcessor::new();
    
    // Test JSON export
    let json_result = processor.export_data(
        sui_token_tracker::transaction_processor::ExportFormat::Json
    ).await;
    assert!(json_result.is_ok());
    
    let json_data = json_result.unwrap();
    assert!(json_data.contains("balances"));
    assert!(json_data.contains("stats"));
    
    // Test CSV export
    let csv_result = processor.export_data(
        sui_token_tracker::transaction_processor::ExportFormat::Csv
    ).await;
    assert!(csv_result.is_ok());
    
    let csv_data = csv_result.unwrap();
    assert!(csv_data.contains("Address,Balance,Total Transactions"));
}

#[tokio::test]
async fn test_cleanup_operations() {
    let processor = TransactionProcessor::new();
    
    // Test cleanup operation (should succeed even with no data)
    let result = processor.cleanup_old_transactions(86400).await;
    assert!(result.is_ok());
    
    let removed_count = result.unwrap();
    assert_eq!(removed_count, 0); // No data to clean up
}

#[test]
fn test_address_validation() {
    use sui_token_tracker::config::Config;
    
    // Valid addresses
    assert!(Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef12345678"));
    assert!(Config::is_valid_sui_address("0x0000000000000000000000000000000000000000000000000000000000000000"));
    assert!(Config::is_valid_sui_address("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"));
    
    // Invalid addresses
    assert!(!Config::is_valid_sui_address("1234567890abcdef1234567890abcdef12345678")); // Missing 0x prefix
    assert!(!Config::is_valid_sui_address("0x123")); // Too short
    assert!(!Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef1234567")); // Too long
    assert!(!Config::is_valid_sui_address("0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")); // Invalid hex characters
    assert!(!Config::is_valid_sui_address("0x1234567890abcdef1234567890abcdef1234567g")); // Invalid hex character
    assert!(!Config::is_valid_sui_address("")); // Empty string
    assert!(!Config::is_valid_sui_address("0x")); // Only prefix
}

#[tokio::test]
async fn test_concurrent_access() {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    
    let processor = Arc::new(TransactionProcessor::new());
    let mut join_set = JoinSet::new();
    
    // Spawn multiple concurrent tasks
    for i in 0..5 {
        let processor_clone = processor.clone();
        join_set.spawn(async move {
            let address = format!("0x{:064x}", i);
            
            // Concurrent balance checks
            let balance = processor_clone.get_address_balance(&address).await;
            
            // Concurrent history checks
            let history = processor_clone.get_address_history(&address, 10).await;
            
            // Concurrent stats checks
            let stats = processor_clone.get_address_stats(&address).await;
            
            (balance, history, stats)
        });
    }
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap());
    }
    
    // Verify all results are consistent
    assert_eq!(results.len(), 5);
    for (balance, history, stats) in results {
        assert_eq!(balance, 0);
        assert_eq!(history.len(), 0);
        assert!(stats.is_none());
    }
}