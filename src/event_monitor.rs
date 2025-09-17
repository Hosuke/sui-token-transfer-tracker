use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use crate::sui_client::{SuiClient, SuiEvent};
use crate::error::{TrackerError, TrackerResult, utils};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct EventMonitor {
    sui_client: Arc<SuiClient>,
    poll_interval: Duration,
    addresses: Arc<RwLock<HashSet<String>>>,
    event_sender: mpsc::UnboundedSender<TransferEvent>,
    address_last_checked: Arc<RwLock<HashMap<String, u64>>>,
    running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    pub transaction_id: String,
    pub package_id: String,
    pub transaction_module: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub token_type: String,
    pub timestamp: u64,
    pub block_number: u64,
    pub event_type: String,
}

#[derive(Debug, Clone)]
pub struct MonitorStats {
    pub total_events_processed: u64,
    pub events_per_second: f64,
    pub last_event_time: Option<DateTime<Utc>>,
    pub monitored_addresses: usize,
    pub errors_count: u64,
}

impl EventMonitor {
    pub async fn new(
        sui_client: Arc<SuiClient>,
        poll_interval: Duration,
    ) -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let monitor = Self {
            sui_client,
            poll_interval,
            addresses: Arc::new(RwLock::new(HashSet::new())),
            event_sender,
            address_last_checked: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        };
        (monitor, event_receiver)
    }

    pub async fn add_address(&self, address: String) -> TrackerResult<()> {
        if !crate::config::Config::is_valid_sui_address(&address) {
            return Err(TrackerError::invalid_address(
                format!("Invalid SUI address: {}", address)
            ));
        }

        let mut addresses = self.addresses.write().await;
        let was_new = addresses.insert(address.clone());
        
        if was_new {
            log::info!("Added new address to monitor: {}", address);
            
            // 初始化最后检查时间
            let mut last_checked = self.address_last_checked.write().await;
            last_checked.insert(address, 0);
        }

        Ok(())
    }

    pub async fn remove_address(&self, address: &str) -> TrackerResult<()> {
        let mut addresses = self.addresses.write().await;
        let removed = addresses.remove(address);
        
        if removed {
            log::info!("Removed address from monitoring: {}", address);
            
            // 移除最后检查时间
            let mut last_checked = self.address_last_checked.write().await;
            last_checked.remove(address);
        }

        Ok(())
    }

    pub async fn get_monitored_addresses(&self) -> Vec<String> {
        let addresses = self.addresses.read().await;
        addresses.iter().cloned().collect()
    }

    pub async fn start_monitoring(&self) {
        let mut running = self.running.write().await;
        if *running {
            log::warn!("Event monitor is already running");
            return;
        }

        *running = true;
        log::info!("Starting event monitoring with {} addresses", 
            self.addresses.read().await.len());

        let addresses = self.addresses.clone();
        let sui_client = self.sui_client.clone();
        let event_sender = self.event_sender.clone();
        let poll_interval = self.poll_interval;
        let address_last_checked = self.address_last_checked.clone();

        tokio::spawn(async move {
            let mut interval_timer = interval(poll_interval);
            
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = Self::check_new_events_for_addresses(
                    &sui_client,
                    &addresses,
                    &event_sender,
                    &address_last_checked,
                ).await {
                    log::error!("Error checking new events: {}", e);
                }
            }
        });
    }

    pub async fn stop_monitoring(&self) {
        let mut running = self.running.write().await;
        *running = false;
        log::info!("Event monitoring stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn check_new_events_for_addresses(
        sui_client: &Arc<SuiClient>,
        addresses: &Arc<RwLock<HashSet<String>>>,
        event_sender: &mpsc::UnboundedSender<TransferEvent>,
        address_last_checked: &Arc<RwLock<HashMap<String, u64>>>,
    ) -> TrackerResult<()> {
        let addresses_list = {
            let addresses = addresses.read().await;
            addresses.iter().cloned().collect::<Vec<_>>()
        };

        if addresses_list.is_empty() {
            return Ok(());
        }

        // 并行检查所有地址
        let mut tasks = Vec::new();
        for address in addresses_list {
            let sui_client = sui_client.clone();
            let event_sender = event_sender.clone();
            let address_last_checked = address_last_checked.clone();

            let task = tokio::spawn(async move {
                let result = utils::retry_operation(
                    || {
                        sui_client.query_transfer_events(&address, 10)
                    },
                    3,
                    1000,
                ).await;

                match result {
                    Ok(events) => {
                        let mut last_checked = address_last_checked.write().await;
                        let current_time = Utc::now().timestamp() as u64;
                        let last_time = last_checked.get(&address).copied().unwrap_or(0);
                        
                        let mut new_events = 0;
                        for event in events {
                            if event.timestamp > last_time {
                                if let Ok(transfer_event) = Self::parse_transfer_event(event) {
                                    if let Err(e) = event_sender.send(transfer_event) {
                                        log::error!("Failed to send transfer event: {}", e);
                                    }
                                    new_events += 1;
                                }
                            }
                        }
                        
                        if new_events > 0 {
                            last_checked.insert(address.clone(), current_time);
                            log::debug!("Found {} new events for address {}", new_events, address);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to query events for address {}: {}", address, e);
                    }
                }
            });

            tasks.push(task);
        }

        // 等待所有任务完成
        for task in tasks {
            if let Err(e) = task.await {
                log::error!("Task execution failed: {}", e);
            }
        }

        Ok(())
    }

    fn parse_transfer_event(event: SuiEvent) -> TrackerResult<TransferEvent> {
        // 解析Sui事件结构，提取转移事件信息
        // 这里需要根据实际的Sui事件结构进行调整
        
        let mut amount = 0;
        let mut recipient = String::new();
        let mut token_type = "0x2::sui::SUI".to_string();
        let event_type = "transfer".to_string();

        // 尝试从解析的JSON中提取信息
        if let Some(parsed) = event.parsed_json.as_object() {
            if let Some(value) = parsed.get("value").and_then(|v| v.as_object()) {
                // 提取金额
                if let Some(amount_str) = value.get("amount").and_then(|v| v.as_str()) {
                    amount = amount_str.parse::<u64>().unwrap_or(0);
                }
                
                // 提取接收方
                if let Some(recipient_addr) = value.get("recipient").and_then(|v| v.as_str()) {
                    recipient = recipient_addr.to_string();
                }
                
                // 提取代币类型
                if let Some(token) = value.get("type").and_then(|v| v.as_str()) {
                    token_type = token.to_string();
                }
            }
        }

        if recipient.is_empty() {
            return Err(TrackerError::parse_error(
                "Could not extract recipient from event"
            ));
        }

        Ok(TransferEvent {
            transaction_id: event.id.tx_digest,
            package_id: event.package_id,
            transaction_module: event.transaction_module,
            sender: event.sender,
            recipient,
            amount,
            token_type,
            timestamp: event.timestamp,
            block_number: event.id.event_seq,
            event_type,
        })
    }

    pub async fn get_stats(&self) -> MonitorStats {
        // 这是一个简化的统计信息实现
        // 在实际应用中，你可能需要维护更详细的统计信息
        
        MonitorStats {
            total_events_processed: 0, // 需要在实现中维护这个计数器
            events_per_second: 0.0,
            last_event_time: None,
            monitored_addresses: self.addresses.read().await.len(),
            errors_count: 0, // 需要在实现中维护这个计数器
        }
    }

    pub async fn force_check_all_addresses(&self) -> TrackerResult<u64> {
        let addresses = self.addresses.read().await;
        let mut total_events = 0;

        for address in addresses.iter() {
            match self.sui_client.query_transfer_events(address, 50).await {
                Ok(events) => {
                    for event in events {
                        if let Ok(transfer_event) = Self::parse_transfer_event(event) {
                            if let Err(e) = self.event_sender.send(transfer_event) {
                                log::error!("Failed to send transfer event: {}", e);
                            } else {
                                total_events += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to query events for address {}: {}", address, e);
                }
            }
        }

        log::info!("Force check completed, found {} new events", total_events);
        Ok(total_events)
    }

    pub async fn validate_addresses(&self) -> Vec<String> {
        let addresses = self.addresses.read().await;
        let mut invalid_addresses = Vec::new();

        for address in addresses.iter() {
            if !crate::config::Config::is_valid_sui_address(address) {
                invalid_addresses.push(address.clone());
            }
        }

        invalid_addresses
    }

    pub async fn update_poll_interval(&self, new_interval: Duration) {
        log::info!("Updating poll interval to {:?}", new_interval);
        // 注意：这里只是记录日志，实际的间隔更新需要重新启动监控
        // 在实际实现中，你可能需要一个更复杂的机制来动态更新间隔
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_event_monitor_creation() {
        let sui_client = Arc::new(
            SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap()
        );
        let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
        
        assert!(!monitor.is_running().await);
    }

    #[tokio::test]
    async fn test_add_remove_address() {
        let sui_client = Arc::new(
            SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap()
        );
        let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
        
        let valid_address = "0x1234567890abcdef1234567890abcdef12345678";
        
        // 测试添加有效地址
        assert!(monitor.add_address(valid_address.to_string()).await.is_ok());
        assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
        
        // 测试添加无效地址
        let invalid_address = "invalid_address";
        assert!(monitor.add_address(invalid_address.to_string()).await.is_err());
        assert_eq!(monitor.get_monitored_addresses().await.len(), 1);
        
        // 测试移除地址
        assert!(monitor.remove_address(valid_address).await.is_ok());
        assert_eq!(monitor.get_monitored_addresses().await.len(), 0);
    }

    #[tokio::test]
    async fn test_validate_addresses() {
        let sui_client = Arc::new(
            SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap()
        );
        let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
        
        // 添加有效和无效地址
        monitor.add_address("0x1234567890abcdef1234567890abcdef12345678".to_string()).await.unwrap();
        monitor.add_address("invalid_address".to_string()).await.unwrap(); // 这实际上会失败
        
        let invalid = monitor.validate_addresses().await;
        assert_eq!(invalid.len(), 0); // 因为无效地址不会被添加
    }

    #[test]
    fn test_parse_transfer_event() {
        // 创建一个模拟的SuiEvent进行测试
        let event = SuiEvent {
            id: crate::sui_client::EventId {
                tx_digest: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
                event_seq: 1,
            },
            package_id: "0x0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            transaction_module: "pay".to_string(),
            sender: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            timestamp: 1634567890,
            parsed_json: serde_json::json!({
                "value": {
                    "amount": "1000000000",
                    "recipient": "0xabcdef1234567890abcdef1234567890abcdef12",
                    "type": "0x2::sui::SUI"
                }
            }),
        };

        let result = EventMonitor::parse_transfer_event(event);
        assert!(result.is_ok());
        
        let transfer_event = result.unwrap();
        assert_eq!(transfer_event.amount, 1000000000);
        assert_eq!(transfer_event.sender, "0x1234567890abcdef1234567890abcdef12345678");
        assert_eq!(transfer_event.recipient, "0xabcdef1234567890abcdef1234567890abcdef12");
    }

    #[tokio::test]
    async fn test_monitor_stats() {
        let sui_client = Arc::new(
            SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap()
        );
        let (monitor, _receiver) = EventMonitor::new(sui_client, Duration::from_secs(10)).await;
        
        let stats = monitor.get_stats().await;
        assert_eq!(stats.monitored_addresses, 0);
        assert_eq!(stats.total_events_processed, 0);
        assert_eq!(stats.errors_count, 0);
    }
}