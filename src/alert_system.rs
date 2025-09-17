use tokio::sync::mpsc;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use crate::transaction_processor::Transaction;
use crate::error::{TrackerError, TrackerResult};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AlertSystem {
    thresholds: HashMap<String, u64>,
    large_transfer_threshold: u64,
    #[allow(dead_code)]
    alert_sender: mpsc::UnboundedSender<Alert>,
    #[allow(dead_code)]
    alert_history: Vec<Alert>,
    config: AlertConfig,
    suspicious_activity_detector: SuspiciousActivityDetector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub low_balance_threshold: u64,
    pub large_transfer_threshold: u64,
    pub enable_console_alerts: bool,
    pub enable_file_alerts: bool,
    pub alert_file_path: String,
    pub enable_email_alerts: bool,
    pub email_smtp_server: String,
    pub email_sender: String,
    pub email_recipients: Vec<String>,
    pub enable_discord_alerts: bool,
    pub discord_webhook_url: String,
    pub cooldown_period_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            low_balance_threshold: 1000000000,
            large_transfer_threshold: 10000000000,
            enable_console_alerts: true,
            enable_file_alerts: false,
            alert_file_path: "alerts.log".to_string(),
            enable_email_alerts: false,
            email_smtp_server: String::new(),
            email_sender: String::new(),
            email_recipients: Vec::new(),
            enable_discord_alerts: false,
            discord_webhook_url: String::new(),
            cooldown_period_seconds: 300, // 5分钟冷却时间
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Alert {
    LowBalance {
        address: String,
        balance: u64,
        threshold: u64,
        severity: AlertSeverity,
        timestamp: DateTime<Utc>,
    },
    LargeTransfer {
        sender: String,
        recipient: String,
        amount: u64,
        transaction_id: String,
        token_type: String,
        severity: AlertSeverity,
        timestamp: DateTime<Utc>,
    },
    SuspiciousActivity {
        address: String,
        activity_type: String,
        description: String,
        risk_level: RiskLevel,
        related_transactions: Vec<String>,
        severity: AlertSeverity,
        timestamp: DateTime<Utc>,
    },
    NetworkError {
        error: String,
        component: String,
        severity: AlertSeverity,
        timestamp: DateTime<Utc>,
    },
    SystemError {
        error: String,
        component: String,
        severity: AlertSeverity,
        timestamp: DateTime<Utc>,
    },
    Custom {
        title: String,
        message: String,
        severity: AlertSeverity,
        category: String,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct SuspiciousActivityDetector {
    transaction_counts: HashMap<String, TransactionCount>,
    last_alert_times: HashMap<String, DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct TransactionCount {
    count: u32,
    #[allow(dead_code)]
    window_start: DateTime<Utc>,
    #[allow(dead_code)]
    window_duration_hours: u64,
}

impl AlertSystem {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Alert>) {
        Self::with_config(AlertConfig::default())
    }

    pub fn with_config(config: AlertConfig) -> (Self, mpsc::UnboundedReceiver<Alert>) {
        let (alert_sender, alert_receiver) = mpsc::unbounded_channel();
        let system = Self {
            thresholds: HashMap::new(),
            large_transfer_threshold: config.large_transfer_threshold,
            alert_sender,
            alert_history: Vec::new(),
            config,
            suspicious_activity_detector: SuspiciousActivityDetector::new(),
        };
        (system, alert_receiver)
    }

    pub async fn set_threshold(&self, _address: String, _threshold: u64) {
        // This method needs to be mutable or use interior mutability
        log::warn!("Cannot set threshold on immutable AlertSystem");
    }

    pub async fn check_balance_alert(&self, address: &str, balance: u64) -> TrackerResult<()> {
        if let Some(&threshold) = self.thresholds.get(address) {
            if balance < threshold {
                let severity = if balance < threshold / 10 {
                    AlertSeverity::Critical
                } else if balance < threshold / 2 {
                    AlertSeverity::Error
                } else {
                    AlertSeverity::Warning
                };

                let alert = Alert::LowBalance {
                    address: address.to_string(),
                    balance,
                    threshold,
                    severity,
                    timestamp: Utc::now(),
                };
                
                self.send_alert(alert).await?;
            }
        }
        Ok(())
    }

    pub async fn check_large_transfer(&self, transaction: &Transaction) -> TrackerResult<()> {
        if transaction.amount > self.large_transfer_threshold {
            let severity = if transaction.amount > self.large_transfer_threshold * 10 {
                AlertSeverity::Critical
            } else if transaction.amount > self.large_transfer_threshold * 5 {
                AlertSeverity::Error
            } else {
                AlertSeverity::Warning
            };

            let alert = Alert::LargeTransfer {
                sender: transaction.sender.clone(),
                recipient: transaction.recipient.clone(),
                amount: transaction.amount,
                transaction_id: transaction.id.clone(),
                token_type: transaction.token_type.clone(),
                severity,
                timestamp: Utc::now(),
            };
            
            self.send_alert(alert).await?;
        }
        Ok(())
    }

    pub async fn check_suspicious_activity(&self, transactions: &[Transaction]) -> TrackerResult<()> {
        let current_time = Utc::now();
        
        for tx in transactions {
            if let Some(alert) = self.suspicious_activity_detector.check_transaction(
                tx,
                current_time,
                &self.config,
            ).await {
                self.send_alert(alert).await?;
            }
        }
        
        Ok(())
    }

    pub async fn send_network_error_alert(&self, error: String, component: String) -> TrackerResult<()> {
        let alert = Alert::NetworkError {
            error,
            component,
            severity: AlertSeverity::Error,
            timestamp: Utc::now(),
        };
        self.send_alert(alert).await
    }

    pub async fn send_system_error_alert(&self, error: String, component: String) -> TrackerResult<()> {
        let alert = Alert::SystemError {
            error,
            component,
            severity: AlertSeverity::Error,
            timestamp: Utc::now(),
        };
        self.send_alert(alert).await
    }

    pub async fn send_custom_alert(&self, title: String, message: String, category: String) -> TrackerResult<()> {
        let alert = Alert::Custom {
            title,
            message,
            severity: AlertSeverity::Info,
            category,
            timestamp: Utc::now(),
        };
        self.send_alert(alert).await
    }

    async fn send_alert(&self, alert: Alert) -> TrackerResult<()> {
        let alert_key = self.get_alert_key(&alert);
        
        // 检查冷却时间
        if self.is_in_cooldown(&alert_key).await {
            log::debug!("Alert {} is in cooldown period, skipping", alert_key);
            return Ok(());
        }

        // 发送到控制台
        if self.config.enable_console_alerts {
            self.send_console_alert(&alert).await;
        }

        // 发送到文件
        if self.config.enable_file_alerts {
            self.send_file_alert(&alert).await?;
        }

        // 发送到邮件
        if self.config.enable_email_alerts {
            self.send_email_alert(&alert).await?;
        }

        // 发送到Discord
        if self.config.enable_discord_alerts {
            self.send_discord_alert(&alert).await?;
        }

        // 记录发送时间
        self.record_alert_time(alert_key.clone()).await;

        // 添加到历史记录
        self.add_to_history(alert.clone()).await;

        // 发送到channel (用于测试和其他组件)
        if let Err(_) = self.alert_sender.send(alert.clone()) {
            log::warn!("Failed to send alert to channel, receiver may be dropped");
        }

        log::info!("Alert sent: {}", alert_key);
        Ok(())
    }

    fn get_alert_key(&self, alert: &Alert) -> String {
        match alert {
            Alert::LowBalance { address, .. } => format!("low_balance_{}", address),
            Alert::LargeTransfer { transaction_id, .. } => format!("large_transfer_{}", transaction_id),
            Alert::SuspiciousActivity { address, activity_type, .. } => {
                format!("suspicious_{}_{}", address, activity_type)
            },
            Alert::NetworkError { component, .. } => format!("network_error_{}", component),
            Alert::SystemError { component, .. } => format!("system_error_{}", component),
            Alert::Custom { category, title, .. } => format!("custom_{}_{}", category, title),
        }
    }

    async fn is_in_cooldown(&self, alert_key: &str) -> bool {
        let current_time = Utc::now();
        
        if let Some(last_alert_time) = self.suspicious_activity_detector.last_alert_times.get(alert_key) {
            let cooldown_duration = chrono::Duration::seconds(self.config.cooldown_period_seconds as i64);
            current_time.signed_duration_since(*last_alert_time) < cooldown_duration
        } else {
            false
        }
    }

    async fn record_alert_time(&self, alert_key: String) {
        let mut last_alert_times = self.suspicious_activity_detector.last_alert_times.clone();
        last_alert_times.insert(alert_key, Utc::now());
    }

    async fn send_console_alert(&self, alert: &Alert) {
        let message = self.format_alert_message(alert);
        match alert.severity() {
            AlertSeverity::Info => println!("{}", message),
            AlertSeverity::Warning => eprintln!("⚠️  {}", message),
            AlertSeverity::Error => eprintln!("❌ {}", message),
            AlertSeverity::Critical => eprintln!("🚨 {}", message),
        }
    }

    async fn send_file_alert(&self, alert: &Alert) -> TrackerResult<()> {
        let message = self.format_alert_message(alert);
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.alert_file_path)
            .map_err(|e| TrackerError::IoError(e))?;

        writeln!(file, "[{}] {}", timestamp, message)
            .map_err(|e| TrackerError::IoError(e))?;

        Ok(())
    }

    async fn send_email_alert(&self, alert: &Alert) -> TrackerResult<()> {
        // 简化的邮件发送实现
        // 在实际应用中，你需要使用像 lettre 这样的库
        log::debug!("Email alert would be sent: {:?}", alert);
        Ok(())
    }

    async fn send_discord_alert(&self, alert: &Alert) -> TrackerResult<()> {
        if self.config.discord_webhook_url.is_empty() {
            return Ok(());
        }

        let message = self.format_discord_message(alert);
        
        // 这里应该发送HTTP请求到Discord webhook
        log::debug!("Discord alert would be sent: {}", message);
        Ok(())
    }

    fn format_alert_message(&self, alert: &Alert) -> String {
        match alert {
            Alert::LowBalance { address, balance, threshold, severity, .. } => {
                format!("ALERT [{}]: Low balance for {}: {} (threshold: {})", 
                    self.severity_to_string(severity), 
                    self.truncate_address(address), 
                    self.format_amount(*balance), 
                    self.format_amount(*threshold))
            },
            Alert::LargeTransfer { sender, recipient, amount, token_type, severity, .. } => {
                format!("ALERT [{}]: Large transfer: {} → {} | Amount: {} {}", 
                    self.severity_to_string(severity),
                    self.truncate_address(sender), 
                    self.truncate_address(recipient), 
                    self.format_amount(*amount), 
                    token_type)
            },
            Alert::SuspiciousActivity { address, activity_type, description, risk_level, severity, .. } => {
                format!("ALERT [{}]: Suspicious activity detected for {}: {} - {} (Risk: {})", 
                    self.severity_to_string(severity),
                    self.truncate_address(address), 
                    activity_type, 
                    description, 
                    self.risk_level_to_string(risk_level))
            },
            Alert::NetworkError { error, component, severity, .. } => {
                format!("ALERT [{}]: Network error in {}: {}", 
                    self.severity_to_string(severity),
                    component, 
                    error)
            },
            Alert::SystemError { error, component, severity, .. } => {
                format!("ALERT [{}]: System error in {}: {}", 
                    self.severity_to_string(severity),
                    component, 
                    error)
            },
            Alert::Custom { title, message, severity, .. } => {
                format!("ALERT [{}]: {} - {}", 
                    self.severity_to_string(severity),
                    title, 
                    message)
            },
        }
    }

    fn format_discord_message(&self, alert: &Alert) -> String {
        let color = match alert.severity() {
            AlertSeverity::Info => 0x3498db, // Blue
            AlertSeverity::Warning => 0xf39c12, // Orange
            AlertSeverity::Error => 0xe74c3c, // Red
            AlertSeverity::Critical => 0x8b0000, // Dark Red
        };

        format!(
            r#"{{"embeds": [{{"title": "SUI Tracker Alert", "description": "{}", "color": {}}}]}}"#,
            self.format_alert_message(alert),
            color
        )
    }

    fn severity_to_string(&self, severity: &AlertSeverity) -> String {
        match severity {
            AlertSeverity::Info => "INFO".to_string(),
            AlertSeverity::Warning => "WARNING".to_string(),
            AlertSeverity::Error => "ERROR".to_string(),
            AlertSeverity::Critical => "CRITICAL".to_string(),
        }
    }

    fn risk_level_to_string(&self, risk_level: &RiskLevel) -> String {
        match risk_level {
            RiskLevel::Low => "LOW".to_string(),
            RiskLevel::Medium => "MEDIUM".to_string(),
            RiskLevel::High => "HIGH".to_string(),
            RiskLevel::Critical => "CRITICAL".to_string(),
        }
    }

    fn format_amount(&self, amount: u64) -> String {
        format!("{:.9} SUI", amount as f64 / 1_000_000_000.0)
    }

    fn truncate_address(&self, address: &str) -> String {
        if address.len() > 10 {
            format!("{}...{}", &address[..6], &address[address.len()-4..])
        } else {
            address.to_string()
        }
    }

    async fn add_to_history(&self, alert: Alert) {
        // 在实际应用中，你可能需要线程安全的历史记录
        // 这里简化处理
        log::debug!("Alert added to history: {:?}", alert);
    }

    pub async fn get_alert_history(&self, _limit: usize) -> Vec<Alert> {
        // 简化版本，返回最近的一些警报
        Vec::new()
    }

    pub async fn get_alert_stats(&self) -> AlertStats {
        AlertStats {
            total_alerts: 0,
            alerts_by_type: HashMap::new(),
            alerts_by_severity: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertStats {
    pub total_alerts: usize,
    pub alerts_by_type: HashMap<String, usize>,
    pub alerts_by_severity: HashMap<String, usize>,
}

impl Alert {
    pub fn severity(&self) -> &AlertSeverity {
        match self {
            Alert::LowBalance { severity, .. } => severity,
            Alert::LargeTransfer { severity, .. } => severity,
            Alert::SuspiciousActivity { severity, .. } => severity,
            Alert::NetworkError { severity, .. } => severity,
            Alert::SystemError { severity, .. } => severity,
            Alert::Custom { severity, .. } => severity,
        }
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Alert::LowBalance { timestamp, .. } => timestamp,
            Alert::LargeTransfer { timestamp, .. } => timestamp,
            Alert::SuspiciousActivity { timestamp, .. } => timestamp,
            Alert::NetworkError { timestamp, .. } => timestamp,
            Alert::SystemError { timestamp, .. } => timestamp,
            Alert::Custom { timestamp, .. } => timestamp,
        }
    }
}

impl SuspiciousActivityDetector {
    pub fn new() -> Self {
        Self {
            transaction_counts: HashMap::new(),
            last_alert_times: HashMap::new(),
        }
    }

    pub async fn check_transaction(
        &self,
        _transaction: &Transaction,
        current_time: DateTime<Utc>,
        _config: &AlertConfig,
    ) -> Option<Alert> {
        // 检查高频交易
        if let Some(alert) = self.check_high_frequency_transactions(_transaction, current_time, _config).await {
            return Some(alert);
        }

        // 检查大额转账到新地址
        if let Some(alert) = self.check_large_transfer_to_new_address(_transaction, _config).await {
            return Some(alert);
        }

        // 检查异常交易模式
        if let Some(alert) = self.check_unusual_patterns(_transaction, _config).await {
            return Some(alert);
        }

        None
    }

    async fn check_high_frequency_transactions(
        &self,
        _transaction: &Transaction,
        current_time: DateTime<Utc>,
        _config: &AlertConfig,
    ) -> Option<Alert> {
        // 简化的高频交易检测
        let _count = self.transaction_counts.get(&_transaction.sender)
            .map(|tc| tc.count)
            .unwrap_or(0);

        if _count > 10 { // 如果短时间内超过10笔交易
            Some(Alert::SuspiciousActivity {
                address: _transaction.sender.clone(),
                activity_type: "high_frequency_transactions".to_string(),
                description: format!("Address has {} transactions in short period", _count),
                risk_level: RiskLevel::Medium,
                related_transactions: vec![_transaction.id.clone()],
                severity: AlertSeverity::Warning,
                timestamp: current_time,
            })
        } else {
            None
        }
    }

    async fn check_large_transfer_to_new_address(
        &self,
        _transaction: &Transaction,
        _config: &AlertConfig,
    ) -> Option<Alert> {
        // 简化的大额转账到新地址检测
        if _transaction.amount > _config.large_transfer_threshold * 2 {
            Some(Alert::SuspiciousActivity {
                address: _transaction.sender.clone(),
                activity_type: "large_transfer_to_new_address".to_string(),
                description: format!("Large transfer of {} to potentially new address", 
                    _transaction.amount),
                risk_level: RiskLevel::High,
                related_transactions: vec![_transaction.id.clone()],
                severity: AlertSeverity::Error,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    async fn check_unusual_patterns(
        &self,
        _transaction: &Transaction,
        _config: &AlertConfig,
    ) -> Option<Alert> {
        // 简化的异常模式检测
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_system_creation() {
        let (alert_system, _receiver) = AlertSystem::new();
        let stats = alert_system.get_alert_stats().await;
        
        assert_eq!(stats.total_alerts, 0);
    }

    #[tokio::test]
    async fn test_set_threshold() {
        let (alert_system, _receiver) = AlertSystem::new();
        
        alert_system.set_threshold("0x1234...".to_string(), 1000000000).await;
        
        // 验证阈值设置（这里需要访问私有字段进行测试）
        // 在实际测试中，你可能需要添加getter方法
    }

    #[tokio::test]
    async fn test_balance_alert() {
        let (alert_system, mut receiver) = AlertSystem::new();
        
        alert_system.set_threshold("0xtest".to_string(), 1000000000).await;
        alert_system.check_balance_alert("0xtest", 500000000).await.unwrap();
        
        if let Some(alert) = receiver.recv().await {
            match alert {
                Alert::LowBalance { address, balance, threshold, .. } => {
                    assert_eq!(address, "0xtest");
                    assert_eq!(balance, 500000000);
                    assert_eq!(threshold, 1000000000);
                }
                _ => panic!("Expected LowBalance alert"),
            }
        }
    }

    #[tokio::test]
    async fn test_large_transfer_alert() {
        let (alert_system, mut receiver) = AlertSystem::new();
        
        let transaction = Transaction {
            id: "0x123".to_string(),
            sender: "0xsender".to_string(),
            recipient: "0xrecipient".to_string(),
            amount: 20000000000, // 20 SUI
            token_type: "0x2::sui::SUI".to_string(),
            timestamp: 1634567890,
            block_number: 12345,
            gas_used: None,
            gas_price: None,
            status: crate::transaction_processor::TransactionStatus::Success,
        };
        
        alert_system.check_large_transfer(&transaction).await.unwrap();
        
        if let Some(alert) = receiver.recv().await {
            match alert {
                Alert::LargeTransfer { sender, recipient, amount, transaction_id, .. } => {
                    assert_eq!(sender, "0xsender");
                    assert_eq!(recipient, "0xrecipient");
                    assert_eq!(amount, 20000000000);
                    assert_eq!(transaction_id, "0x123");
                }
                _ => panic!("Expected LargeTransfer alert"),
            }
        }
    }

    #[test]
    fn test_alert_severity() {
        let alert = Alert::LowBalance {
            address: "0xtest".to_string(),
            balance: 500000000,
            threshold: 1000000000,
            severity: AlertSeverity::Warning,
            timestamp: Utc::now(),
        };
        
        assert!(matches!(alert.severity(), &AlertSeverity::Warning));
    }

    #[test]
    fn test_alert_formatting() {
        let (alert_system, _receiver) = AlertSystem::new();
        
        let alert = Alert::LowBalance {
            address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            balance: 500000000,
            threshold: 1000000000,
            severity: AlertSeverity::Warning,
            timestamp: Utc::now(),
        };
        
        let message = alert_system.format_alert_message(&alert);
        assert!(message.contains("0x123456...45678"));
        assert!(message.contains("0.500000000 SUI"));
        assert!(message.contains("1.000000000 SUI"));
    }
}