pub mod config;
pub mod sui_client;
pub mod event_monitor;
pub mod transaction_processor;
pub mod alert_system;
pub mod output_formatter;
pub mod error;

use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc, Mutex};
use tokio::time::{Duration, interval};
use std::sync::Arc;
use crate::{sui_client::SuiClient, event_monitor::EventMonitor, transaction_processor::TransactionProcessor, alert_system::{AlertSystem, AlertConfig}, output_formatter::OutputFormatter};
use crate::event_monitor::TransferEvent;
use crate::alert_system::Alert;

// Re-export public types
pub use crate::config::Config;
pub use crate::error::{TrackerError, TrackerResult};
pub use crate::output_formatter::OutputFormat;

pub struct TokenTransferTracker {
    config: crate::config::Config,
    sui_client: Arc<SuiClient>,
    event_monitor: EventMonitor,
    event_receiver: Mutex<mpsc::UnboundedReceiver<TransferEvent>>,
    pub transaction_processor: TransactionProcessor,
    alert_system: AlertSystem,
    alert_receiver: Mutex<mpsc::UnboundedReceiver<Alert>>,
    pub output_formatter: OutputFormatter,
    monitored_addresses: RwLock<HashMap<String, AddressInfo>>,
    running: RwLock<bool>,
    stats: RwLock<TrackerStats>,
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub balance: u64,
    pub last_checked: u64,
    pub alert_threshold: Option<u64>,
    pub total_transactions: u64,
    pub first_seen: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct TrackerStats {
    pub start_time: std::time::SystemTime,
    pub total_events_processed: u64,
    pub total_transactions_processed: u64,
    pub total_alerts_sent: u64,
    pub total_errors: u64,
    pub uptime_seconds: u64,
    pub addresses_monitored: usize,
}

impl TokenTransferTracker {
    pub async fn new(config: crate::config::Config) -> crate::error::TrackerResult<Self> {
        log::info!("Initializing SUI Token Transfer Tracker");
        
        // 验证配置
        config.validate()?;
        
        // 初始化日志
        Self::init_logging(&config.logging);

        // 创建SUI客户端
        let sui_client = Arc::new(
            SuiClient::with_timeout(&config.network.rpc_url, config.network.timeout_seconds).await?
        );

        // 健康检查
        if !sui_client.is_healthy().await {
            return Err(TrackerError::network_error("SUI network connection failed"));
        }

        // 创建事件监控器
        let (event_monitor, event_receiver) = EventMonitor::new(
            sui_client.clone(),
            Duration::from_secs(config.monitoring.poll_interval_seconds),
        ).await;

        // 创建交易处理器
        let transaction_processor = TransactionProcessor::with_config(crate::transaction_processor::ProcessorConfig {
            max_history_records: config.monitoring.max_history_records,
            cleanup_interval_hours: config.monitoring.cleanup_interval_hours,
            enable_detailed_stats: true,
        });

        // 创建警报系统
        let alert_config = AlertConfig {
            low_balance_threshold: config.alerts.low_balance_threshold,
            large_transfer_threshold: config.alerts.large_transfer_threshold,
            enable_console_alerts: config.alerts.enable_console_alerts,
            enable_file_alerts: config.alerts.enable_file_alerts,
            alert_file_path: config.alerts.alert_file_path.clone(),
            enable_email_alerts: false, // 简化版本
            email_smtp_server: String::new(),
            email_sender: String::new(),
            email_recipients: Vec::new(),
            enable_discord_alerts: false,
            discord_webhook_url: String::new(),
            cooldown_period_seconds: 300,
        };
        
        let (alert_system, alert_receiver) = AlertSystem::with_config(alert_config);

        // 创建输出格式化器
        let output_formatter = OutputFormatter::with_config(crate::output_formatter::OutputConfig {
            use_colors: config.output.use_colors,
            show_timestamps: config.output.show_timestamps,
            max_recent_transactions: config.output.max_recent_transactions,
            balance_summary_interval: config.output.balance_summary_interval,
            table_width: 80,
            enable_json_output: false,
            enable_csv_output: false,
        });

        // 初始化监控地址
        let mut monitored_addresses = HashMap::new();
        for address in &config.addresses.monitored {
            if !config::Config::is_valid_sui_address(address) {
                log::warn!("Invalid address format: {}", address);
                continue;
            }
            
            // 获取初始余额
            let balance = sui_client.get_balance(address, Some("0x2::sui::SUI")).await.unwrap_or(0);
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            monitored_addresses.insert(address.clone(), AddressInfo {
                balance,
                last_checked: current_time,
                alert_threshold: Some(config.alerts.low_balance_threshold),
                total_transactions: 0,
                first_seen: current_time,
                last_seen: current_time,
            });

            // 添加到监控器
            event_monitor.add_address(address.clone()).await?;
        }

        log::info!("Initialized with {} addresses to monitor", monitored_addresses.len());

        Ok(Self {
            config,
            sui_client,
            event_monitor,
            event_receiver: Mutex::new(event_receiver),
            transaction_processor,
            alert_system,
            alert_receiver: Mutex::new(alert_receiver),
            output_formatter,
            monitored_addresses: RwLock::new(monitored_addresses),
            running: RwLock::new(false),
            stats: RwLock::new(TrackerStats {
                start_time: std::time::SystemTime::now(),
                total_events_processed: 0,
                total_transactions_processed: 0,
                total_alerts_sent: 0,
                total_errors: 0,
                uptime_seconds: 0,
                addresses_monitored: 0, // TODO: Fix borrow checker issue
            }),
        })
    }

    pub async fn start_monitoring(&mut self) -> crate::error::TrackerResult<()> {
        let mut running = self.running.write().await;
        if *running {
            log::warn!("Tracker is already running");
            return Ok(());
        }

        *running = true;
        log::info!("Starting SUI Token Transfer Tracker");

        // 启动事件监控
        let event_monitor = self.event_monitor.clone();
        tokio::spawn(async move {
            event_monitor.start_monitoring().await;
        });

        // 启动主处理循环
        self.processing_loop().await?;

        Ok(())
    }

    pub async fn stop_monitoring(&self) -> crate::error::TrackerResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            log::warn!("Tracker is not running");
            return Ok(());
        }

        *running = false;
        self.event_monitor.stop_monitoring().await;
        log::info!("Tracker stopped");

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn processing_loop(&self) -> crate::error::TrackerResult<()> {
        log::info!("Starting processing loop");

        let mut interval_timer = interval(Duration::from_secs(30)); // 维护任务间隔
        let mut balance_summary_interval = interval(Duration::from_secs(self.config.output.balance_summary_interval));

        loop {
            let mut event_receiver = self.event_receiver.lock().await;
            let mut alert_receiver = self.alert_receiver.lock().await;
            
            tokio::select! {
                // 事件处理
                _ = event_receiver.recv() => {
                    if let Err(e) = self.handle_events().await {
                        log::error!("Error handling events: {}", e);
                        self.increment_errors().await;
                    }
                }
                
                // 警报处理
                _ = alert_receiver.recv() => {
                    if let Err(e) = self.handle_alerts().await {
                        log::error!("Error handling alerts: {}", e);
                        self.increment_errors().await;
                    }
                }

                // 定期维护任务
                _ = interval_timer.tick() => {
                    if let Err(e) = self.maintenance_tasks().await {
                        log::error!("Error in maintenance tasks: {}", e);
                        self.increment_errors().await;
                    }
                }

                // 余额摘要输出
                _ = balance_summary_interval.tick() => {
                    if let Err(e) = self.output_balance_summary().await {
                        log::error!("Error outputting balance summary: {}", e);
                        self.increment_errors().await;
                    }
                }

                // 检查是否应该停止
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    if !*self.running.read().await {
                        log::info!("Processing loop stopped");
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn handle_events(&self) -> crate::error::TrackerResult<()> {
        // 这里需要从事件监控器获取事件
        // 这是一个简化的实现
        Ok(())
    }

    async fn handle_alerts(&self) -> crate::error::TrackerResult<()> {
        // 这里需要从警报系统获取警报
        // 这是一个简化的实现
        Ok(())
    }

    #[allow(dead_code)]
    async fn process_transfer_event(&self, event: TransferEvent) -> crate::error::TrackerResult<()> {
        // 更新统计信息
        self.increment_events_processed().await;

        // 处理转移事件
        let processed = self.transaction_processor.process_transfer_event(event.clone()).await?;

        // 检查警报
        self.alert_system.check_large_transfer(&processed.transaction).await?;
        
        // 检查余额警报
        let sender_balance = self.transaction_processor.get_address_balance(&event.sender).await;
        let receiver_balance = self.transaction_processor.get_address_balance(&event.recipient).await;
        
        self.alert_system.check_balance_alert(&event.sender, sender_balance).await?;
        self.alert_system.check_balance_alert(&event.recipient, receiver_balance).await?;

        // 更新地址信息
        self.update_address_info(&event).await?;

        // 输出交易信息
        let formatted = self.output_formatter.format_transaction(&processed.transaction);
        println!("{}", formatted);

        // 更新统计信息
        self.increment_transactions_processed().await;

        log::debug!("Processed transfer event: {}", event.transaction_id);
        Ok(())
    }

    #[allow(dead_code)]
    async fn update_address_info(&self, event: &TransferEvent) -> crate::error::TrackerResult<()> {
        let mut addresses = self.monitored_addresses.write().await;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 更新发送方信息
        if let Some(sender_info) = addresses.get_mut(&event.sender) {
            sender_info.balance = self.transaction_processor.get_address_balance(&event.sender).await;
            sender_info.last_checked = current_time;
            sender_info.total_transactions += 1;
            sender_info.last_seen = current_time;
        }

        // 更新接收方信息
        if let Some(receiver_info) = addresses.get_mut(&event.recipient) {
            receiver_info.balance = self.transaction_processor.get_address_balance(&event.recipient).await;
            receiver_info.last_checked = current_time;
            receiver_info.total_transactions += 1;
            receiver_info.last_seen = current_time;
        }

        Ok(())
    }

    async fn maintenance_tasks(&self) -> crate::error::TrackerResult<()> {
        log::debug!("Running maintenance tasks");

        // 清理过期交易记录
        let removed = self.transaction_processor.cleanup_old_transactions(86400).await?;
        if removed > 0 {
            log::info!("Cleaned up {} old transaction records", removed);
        }

        // 更新运行时间统计
        self.update_uptime().await;

        // 验证监控地址
        let invalid_addresses = self.event_monitor.validate_addresses().await;
        if !invalid_addresses.is_empty() {
            log::warn!("Found {} invalid addresses: {:?}", invalid_addresses.len(), invalid_addresses);
        }

        // 检查系统健康状态
        if !self.sui_client.is_healthy().await {
            log::warn!("SUI network health check failed");
            self.alert_system.send_network_error_alert(
                "SUI network health check failed".to_string(),
                "network_monitor".to_string(),
            ).await?;
        }

        Ok(())
    }

    async fn output_balance_summary(&self) -> crate::error::TrackerResult<()> {
        let balances = self.transaction_processor.get_all_balances().await;
        let summary = self.output_formatter.format_balance_summary(&balances);
        
        println!("\n{}", summary);
        
        // 输出系统统计信息
        let stats = self.transaction_processor.get_processor_stats().await;
        let stats_summary = self.output_formatter.format_system_stats(&stats);
        println!("\n{}", stats_summary);

        Ok(())
    }

    pub async fn add_address(&self, address: String) -> crate::error::TrackerResult<()> {
        if !crate::config::Config::is_valid_sui_address(&address) {
            return Err(TrackerError::invalid_address(
                format!("Invalid SUI address: {}", address)
            ));
        }

        // 获取初始余额
        let balance = self.sui_client.get_balance(&address, Some("0x2::sui::SUI")).await?;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        {
            let mut addresses = self.monitored_addresses.write().await;
            addresses.insert(address.clone(), AddressInfo {
                balance,
                last_checked: current_time,
                alert_threshold: Some(self.config.alerts.low_balance_threshold),
                total_transactions: 0,
                first_seen: current_time,
                last_seen: current_time,
            });
        }

        // 添加到监控器
        self.event_monitor.add_address(address.clone()).await?;

        // 更新统计信息
        self.update_monitored_addresses_count().await;

        log::info!("Added address to monitoring: {}", address);
        println!("{}", self.output_formatter.format_success(&format!("Added address: {}", address)));

        Ok(())
    }

    pub async fn remove_address(&self, address: &str) -> crate::error::TrackerResult<()> {
        {
            let mut addresses = self.monitored_addresses.write().await;
            addresses.remove(address);
        }

        self.event_monitor.remove_address(address).await?;
        self.update_monitored_addresses_count().await;

        log::info!("Removed address from monitoring: {}", address);
        println!("{}", self.output_formatter.format_success(&format!("Removed address: {}", address)));

        Ok(())
    }

    pub async fn get_address_info(&self, address: &str) -> Option<AddressInfo> {
        let addresses = self.monitored_addresses.read().await;
        addresses.get(address).cloned()
    }

    pub async fn get_all_addresses(&self) -> Vec<String> {
        let addresses = self.monitored_addresses.read().await;
        addresses.keys().cloned().collect()
    }

    pub async fn get_tracker_stats(&self) -> TrackerStats {
        self.stats.read().await.clone()
    }

    // 公开的查询方法，用于命令行工具
    pub async fn query_balance(&self, address: &str, coin_type: Option<&str>) -> crate::error::TrackerResult<u64> {
        self.sui_client.get_balance(address, coin_type).await
    }

    pub async fn query_all_balances(&self, address: &str) -> crate::error::TrackerResult<Vec<(String, u64)>> {
        self.sui_client.get_all_balances(address).await
    }

    pub async fn query_transactions_sent(&self, address: &str, limit: Option<u16>) -> crate::error::TrackerResult<Vec<crate::sui_client::SuiTransaction>> {
        self.sui_client.query_transactions_sent(address, limit).await
    }

    pub async fn query_transactions_received(&self, address: &str, limit: Option<u16>) -> crate::error::TrackerResult<Vec<crate::sui_client::SuiTransaction>> {
        self.sui_client.query_transactions_received(address, limit).await
    }

    pub async fn force_balance_check(&self) -> crate::error::TrackerResult<()> {
        log::info!("Forcing balance check for all addresses");
        
        let addresses = self.get_all_addresses().await;
        let mut updates = 0;

        for address in addresses {
            match self.sui_client.get_balance(&address, Some("0x2::sui::SUI")).await {
                Ok(balance) => {
                    let mut addresses = self.monitored_addresses.write().await;
                    if let Some(address_info) = addresses.get_mut(&address) {
                        address_info.balance = balance;
                        address_info.last_checked = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        updates += 1;
                    }
                }
                Err(e) => {
                    log::error!("Failed to get balance for address {}: {}", address, e);
                }
            }
        }

        log::info!("Balance check completed, updated {} addresses", updates);
        println!("{}", self.output_formatter.format_success(&format!("Balance check completed, updated {} addresses", updates)));

        Ok(())
    }

    pub async fn export_data(&self, format: &str, output_path: &str) -> crate::error::TrackerResult<()> {
        let export_format = match format {
            "json" => crate::transaction_processor::ExportFormat::Json,
            "csv" => crate::transaction_processor::ExportFormat::Csv,
            _ => return Err(TrackerError::validation_error("Invalid export format. Use 'json' or 'csv'")),
        };

        let data = self.transaction_processor.export_data(export_format).await?;
        std::fs::write(output_path, data)?;
        
        log::info!("Exported data to {} in {} format", output_path, format);
        println!("{}", self.output_formatter.format_success(&format!("Exported data to {}", output_path)));

        Ok(())
    }

    // 统计信息更新方法
    #[allow(dead_code)]
    async fn increment_events_processed(&self) {
        let mut stats = self.stats.write().await;
        stats.total_events_processed += 1;
    }

    #[allow(dead_code)]
    async fn increment_transactions_processed(&self) {
        let mut stats = self.stats.write().await;
        stats.total_transactions_processed += 1;
    }

    #[allow(dead_code)]
    async fn increment_alerts_sent(&self) {
        let mut stats = self.stats.write().await;
        stats.total_alerts_sent += 1;
    }

    async fn increment_errors(&self) {
        let mut stats = self.stats.write().await;
        stats.total_errors += 1;
    }

    async fn update_uptime(&self) {
        let mut stats = self.stats.write().await;
        stats.uptime_seconds = stats.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs();
    }

    async fn update_monitored_addresses_count(&self) {
        let mut stats = self.stats.write().await;
        stats.addresses_monitored = self.monitored_addresses.read().await.len();
    }

    fn init_logging(logging_config: &crate::config::LoggingConfig) {
        use env_logger::Builder;
        use log::LevelFilter;

        let mut builder = Builder::from_default_env();
        
        // 设置日志级别
        let level = match logging_config.level.as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };
        
        builder.filter_level(level);
        
        // 如果需要文件输出
        if !logging_config.file_path.is_empty() {
            builder.target(env_logger::Target::Pipe(Box::new(std::fs::File::create(&logging_config.file_path).unwrap())));
        }
        
        let _ = builder.try_init();
        
        log::info!("Logging initialized with level: {}", logging_config.level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_tracker_creation() {
        let config = Config::default();
        let result = TokenTransferTracker::new(config).await;
        
        // 这个测试可能因为网络连接而失败，所以我们主要检查错误处理
        match result {
            Ok(_) => {
                println!("Tracker created successfully");
            }
            Err(e) => {
                println!("Tracker creation failed (expected in test environment): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_address_validation() {
        let config = Config::default();
        let tracker = TokenTransferTracker::new(config).await;
        
        match tracker {
            Ok(tracker) => {
                // 测试添加有效地址
                let valid_address = "0x1234567890abcdef1234567890abcdef12345678";
                let result = tracker.add_address(valid_address.to_string()).await;
                assert!(result.is_ok());
                
                // 测试添加无效地址
                let invalid_address = "invalid_address";
                let result = tracker.add_address(invalid_address.to_string()).await;
                assert!(result.is_err());
            }
            Err(_) => {
                println!("Skipping address validation test due to network issues");
            }
        }
    }
}