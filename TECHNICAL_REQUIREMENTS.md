# SUI Token Transfer Tracker - 技术需求文档

## 项目概述

**项目名称**: SUI Token Transfer Tracker  
**项目类型**: Rust + Sui 区块链监控工具  
**目标用户**: Sui代币持有者、交易员、开发者  
**开发时间**: 1天（黑客松项目）

---

## 1. 项目架构设计

### 1.1 系统架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        SUI Token Transfer Tracker                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   CLI Interface │  │  Config Manager │  │  Output Handler │  │
│  │   (clap)        │  │  (serde_toml)   │  │  (formatter)    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│           │                     │                     │         │
│           └─────────────────────┼─────────────────────┘         │
│                                 │                               │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                   Core Application Logic                     │  │
│  │                                                             │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│  │
│  │  │ Event Monitor   │  │  Alert System   │  │  Data Processor ││  │
│  │  │ (tokio::timer)  │  │  (thresholds)   │  │  (calculation) ││  │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘│  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                 │                               │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    SUI Client Layer                          │  │
│  │                                                             │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│  │
│  │  │  SuiClient      │  │  Event Query    │  │  WebSocket      ││  │
│  │  │  (sui-sdk)      │  │  (RPC calls)    │  │  (real-time)    ││  │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘│  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                 │                               │
└─────────────────────────────────┼───────────────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   SUI Network   │
                    │  (Blockchain)   │
                    └─────────────────┘
```

### 1.2 核心组件说明

#### 1.2.1 主应用程序结构
- **入口点**: `src/main.rs`
- **核心模块**: `src/lib.rs`
- **功能模块**: 分布在 `src/` 目录下

#### 1.2.2 主要模块职责
- `sui_client`: SUI网络客户端封装
- `event_monitor`: 事件监控器
- `transaction_processor`: 交易处理器
- `alert_system`: 警报系统
- `output_formatter`: 输出格式化器
- `config`: 配置管理

---

## 2. 实现步骤

### 2.1 Phase 1: 基础架构搭建 (4小时)

#### 步骤1: 创建项目结构
```bash
# 创建新的Rust项目
cargo new sui-token-transfer-tracker
cd sui-token-transfer-tracker

# 创建模块目录
mkdir -p src/{sui_client,event_monitor,transaction_processor,alert_system,output_formatter,config}

# 创建配置和文档目录
mkdir config docs
```

#### 步骤2: 配置Cargo.toml
```toml
[package]
name = "sui-token-transfer-tracker"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "Real-time SUI token transfer monitoring tool"

[dependencies]
# Sui SDK
sui-sdk = "0.32.0"
sui-types = "0.32.0"

# 异步运行时
tokio = { version = "1.28", features = ["full"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_toml = "0.8"

# 命令行解析
clap = { version = "4.3", features = ["derive"] }

# HTTP客户端
reqwest = { version = "0.11", features = ["json"] }

# 错误处理
anyhow = "1.0"
thiserror = "1.0"

# 日志
log = "0.4"
env_logger = "0.10"

# 时间处理
chrono = { version = "0.4", features = ["serde"] }

# 数据处理
tokio-stream = "0.1"
futures = "0.3"

# 配置管理
config = "0.13"

# 工具
uuid = { version = "1.3", features = ["v4"] }
base64 = "0.21"
hex = "0.4"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.11"
```

#### 步骤3: 实现基础模块结构
```rust
// src/lib.rs
pub mod config;
pub mod sui_client;
pub mod event_monitor;
pub mod transaction_processor;
pub mod alert_system;
pub mod output_formatter;
pub mod error;

use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::{config::Config, sui_client::SuiClient, event_monitor::EventMonitor};

pub struct TokenTransferTracker {
    config: Config,
    sui_client: SuiClient,
    event_monitor: EventMonitor,
    monitored_addresses: RwLock<HashMap<String, AddressInfo>>,
}

pub struct AddressInfo {
    pub balance: u64,
    pub last_checked: u64,
    pub alert_threshold: Option<u64>,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub token_type: String,
    pub timestamp: u64,
    pub block_number: u64,
}
```

### 2.2 Phase 2: 核心功能实现 (4小时)

#### 步骤4: 实现SUI客户端
```rust
// src/sui_client.rs
use sui_sdk::SuiClient as SuiSdkClient;
use sui_sdk::rpc_types::{SuiEvent, EventFilter, SuiObjectData};
use crate::error::{TrackerError, TrackerResult};

pub struct SuiClient {
    client: SuiSdkClient,
    network_url: String,
}

impl SuiClient {
    pub async fn new(network_url: &str) -> TrackerResult<Self> {
        let client = SuiSdkClient::new(network_url)
            .await
            .map_err(|e| TrackerError::SuiClientError(e))?;
        
        Ok(Self {
            client,
            network_url: network_url.to_string(),
        })
    }

    pub async fn query_transfer_events(
        &self,
        address: &str,
        limit: u32,
    ) -> TrackerResult<Vec<SuiEvent>> {
        let filter = EventFilter::Sender(address.parse().map_err(|_| {
            TrackerError::ParseError("Invalid address format".into())
        })?);
        
        let events = self.client.query_events(filter, limit)
            .await
            .map_err(|e| TrackerError::SuiClientError(e))?;
        
        Ok(events)
    }

    pub async fn get_balance(&self, address: &str) -> TrackerResult<u64> {
        let balance = self.client.get_balance(address)
            .await
            .map_err(|e| TrackerError::SuiClientError(e))?;
        
        Ok(balance)
    }

    pub async fn get_latest_checkpoint(&self) -> TrackerResult<u64> {
        let checkpoint = self.client.get_latest_checkpoint()
            .await
            .map_err(|e| TrackerError::SuiClientError(e))?;
        
        Ok(checkpoint.sequence_number)
    }

    pub async fn subscribe_to_events(&self, address: &str) -> TrackerResult<EventStream> {
        let filter = EventFilter::Sender(address.parse().map_err(|_| {
            TrackerError::ParseError("Invalid address format".into())
        })?);
        
        let stream = self.client.subscribe_events(filter)
            .await
            .map_err(|e| TrackerError::SuiClientError(e))?;
        
        Ok(stream)
    }
}
```

#### 步骤5: 实现事件监控器
```rust
// src/event_monitor.rs
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::collections::HashSet;
use std::sync::Arc;
use crate::sui_client::SuiClient;
use crate::error::{TrackerError, TrackerResult};

pub struct EventMonitor {
    sui_client: Arc<SuiClient>,
    poll_interval: Duration,
    addresses: RwLock<HashSet<String>>,
    event_sender: mpsc::UnboundedSender<TransferEvent>,
}

#[derive(Debug, Clone)]
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
            addresses: RwLock::new(HashSet::new()),
            event_sender,
        };
        (monitor, event_receiver)
    }

    pub async fn add_address(&self, address: String) -> TrackerResult<()> {
        self.addresses.write().await.insert(address);
        Ok(())
    }

    pub async fn remove_address(&self, address: &str) -> TrackerResult<()> {
        self.addresses.write().await.remove(address);
        Ok(())
    }

    pub async fn start_monitoring(&self) {
        let mut interval_timer = interval(self.poll_interval);
        
        loop {
            interval_timer.tick().await;
            if let Err(e) = self.check_new_events().await {
                log::error!("Error checking new events: {}", e);
            }
        }
    }

    async fn check_new_events(&self) -> TrackerResult<()> {
        let addresses = self.addresses.read().await;
        
        for address in addresses.iter() {
            match self.sui_client.query_transfer_events(address, 10).await {
                Ok(events) => {
                    for event in events {
                        if let Ok(transfer_event) = self.parse_transfer_event(event) {
                            let _ = self.event_sender.send(transfer_event);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error querying events for address {}: {}", address, e);
                }
            }
        }
        
        Ok(())
    }

    fn parse_transfer_event(&self, event: SuiEvent) -> TrackerResult<TransferEvent> {
        // 解析Sui事件结构，提取转移事件信息
        // 这里需要根据Sui事件的实际结构进行调整
        
        Ok(TransferEvent {
            transaction_id: event.id.to_string(),
            package_id: "package_id".to_string(), // 从event中提取
            transaction_module: "module_name".to_string(), // 从event中提取
            sender: "sender_address".to_string(), // 从event中提取
            recipient: "recipient_address".to_string(), // 从event中提取
            amount: 1000000, // 从event中提取
            token_type: "0x2::sui::SUI".to_string(), // 从event中提取
            timestamp: event.timestamp,
            block_number: event.checkpoint,
        })
    }
}
```

#### 步骤6: 实现交易处理器
```rust
// src/transaction_processor.rs
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::event_monitor::TransferEvent;
use crate::error::{TrackerError, TrackerResult};

pub struct TransactionProcessor {
    address_balances: RwLock<HashMap<String, u64>>,
    transaction_history: RwLock<HashMap<String, Vec<Transaction>>>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub token_type: String,
    pub timestamp: u64,
    pub block_number: u64,
}

#[derive(Debug)]
pub struct ProcessedTransaction {
    pub transaction: Transaction,
    pub sender_balance_change: i64,
    pub receiver_balance_change: i64,
}

impl TransactionProcessor {
    pub fn new() -> Self {
        Self {
            address_balances: RwLock::new(HashMap::new()),
            transaction_history: RwLock::new(HashMap::new()),
        }
    }

    pub async fn process_transfer_event(&self, event: TransferEvent) -> TrackerResult<ProcessedTransaction> {
        let mut balances = self.address_balances.write().await;
        let mut history = self.transaction_history.write().await;

        // 更新发送方余额
        let sender_balance = balances.entry(event.sender.clone()).or_insert(0);
        let sender_old_balance = *sender_balance;
        *sender_balance = sender_balance.saturating_sub(event.amount);

        // 更新接收方余额
        let receiver_balance = balances.entry(event.recipient.clone()).or_insert(0);
        let receiver_old_balance = *receiver_balance;
        *receiver_balance = receiver_balance.saturating_add(event.amount);

        // 创建交易记录
        let transaction = Transaction {
            id: event.transaction_id,
            sender: event.sender.clone(),
            recipient: event.recipient.clone(),
            amount: event.amount,
            token_type: event.token_type,
            timestamp: event.timestamp,
            block_number: event.block_number,
        };

        // 添加到历史记录
        history.entry(event.sender.clone())
            .or_insert_with(Vec::new)
            .push(transaction.clone());
        
        history.entry(event.recipient.clone())
            .or_insert_with(Vec::new)
            .push(transaction.clone());

        Ok(ProcessedTransaction {
            transaction,
            sender_balance_change: -(event.amount as i64),
            receiver_balance_change: event.amount as i64,
        })
    }

    pub async fn get_address_balance(&self, address: &str) -> u64 {
        let balances = self.address_balances.read().await;
        balances.get(address).copied().unwrap_or(0)
    }

    pub async fn get_address_history(&self, address: &str, limit: u32) -> Vec<Transaction> {
        let history = self.transaction_history.read().await;
        history.get(address)
            .map(|transactions| {
                let mut txs = transactions.clone();
                txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                txs.into_iter().take(limit as usize).collect()
            })
            .unwrap_or_default()
    }

    pub async fn get_all_balances(&self) -> HashMap<String, u64> {
        let balances = self.address_balances.read().await;
        balances.clone()
    }

    pub async fn cleanup_old_transactions(&self, max_age_seconds: u64) {
        let current_time = chrono::Utc::now().timestamp() as u64;
        let mut history = self.transaction_history.write().await;
        
        for (_, transactions) in history.iter_mut() {
            transactions.retain(|tx| {
                current_time.saturating_sub(tx.timestamp) <= max_age_seconds
            });
        }
    }
}
```

### 2.3 Phase 3: 增强功能实现 (4小时)

#### 步骤7: 实现警报系统
```rust
// src/alert_system.rs
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::transaction_processor::Transaction;
use crate::error::{TrackerError, TrackerResult};

pub struct AlertSystem {
    thresholds: RwLock<HashMap<String, u64>>,
    large_transfer_threshold: u64,
    alert_sender: mpsc::UnboundedSender<Alert>,
}

#[derive(Debug, Clone)]
pub enum Alert {
    LowBalance {
        address: String,
        balance: u64,
        threshold: u64,
    },
    LargeTransfer {
        sender: String,
        recipient: String,
        amount: u64,
        transaction_id: String,
    },
    SuspiciousActivity {
        address: String,
        activity_type: String,
        description: String,
    },
    NetworkError {
        error: String,
    },
}

impl AlertSystem {
    pub fn new(large_transfer_threshold: u64) -> (Self, mpsc::UnboundedReceiver<Alert>) {
        let (alert_sender, alert_receiver) = mpsc::unbounded_channel();
        let system = Self {
            thresholds: RwLock::new(HashMap::new()),
            large_transfer_threshold,
            alert_sender,
        };
        (system, alert_receiver)
    }

    pub async fn set_threshold(&self, address: String, threshold: u64) {
        self.thresholds.write().await.insert(address, threshold);
    }

    pub async fn check_balance_alert(&self, address: &str, balance: u64) -> TrackerResult<()> {
        let thresholds = self.thresholds.read().await;
        if let Some(&threshold) = thresholds.get(address) {
            if balance < threshold {
                let alert = Alert::LowBalance {
                    address: address.to_string(),
                    balance,
                    threshold,
                };
                let _ = self.alert_sender.send(alert);
            }
        }
        Ok(())
    }

    pub async fn check_large_transfer(&self, transaction: &Transaction) -> TrackerResult<()> {
        if transaction.amount > self.large_transfer_threshold {
            let alert = Alert::LargeTransfer {
                sender: transaction.sender.clone(),
                recipient: transaction.recipient.clone(),
                amount: transaction.amount,
                transaction_id: transaction.id.clone(),
            };
            let _ = self.alert_sender.send(alert);
        }
        Ok(())
    }

    pub async fn check_suspicious_activity(&self, transactions: &[Transaction]) -> TrackerResult<()> {
        // 检查高频交易模式
        let mut tx_counts: HashMap<String, u32> = HashMap::new();
        
        for tx in transactions {
            *tx_counts.entry(tx.sender.clone()).or_insert(0) += 1;
        }

        for (address, count) in tx_counts {
            if count > 10 { // 如果短时间内超过10笔交易
                let alert = Alert::SuspiciousActivity {
                    address: address.clone(),
                    activity_type: "high_frequency_transactions".to_string(),
                    description: format!("Address {} has {} transactions in short period", address, count),
                };
                let _ = self.alert_sender.send(alert);
            }
        }

        Ok(())
    }

    pub async fn send_network_error_alert(&self, error: String) -> TrackerResult<()> {
        let alert = Alert::NetworkError { error };
        let _ = self.alert_sender.send(alert);
        Ok(())
    }
}
```

#### 步骤8: 实现输出格式化器
```rust
// src/output_formatter.rs
use crate::transaction_processor::Transaction;
use crate::alert_system::Alert;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct OutputFormatter {
    use_colors: bool,
    show_timestamps: bool,
}

impl OutputFormatter {
    pub fn new(use_colors: bool, show_timestamps: bool) -> Self {
        Self {
            use_colors,
            show_timestamps,
        }
    }

    pub fn format_transaction(&self, transaction: &Transaction) -> String {
        let timestamp = if self.show_timestamps {
            let dt = DateTime::from_timestamp(transaction.timestamp as i64, 0)
                .unwrap_or_default();
            format!("[{}] ", dt.format("%Y-%m-%d %H:%M:%S"))
        } else {
            String::new()
        };

        let amount_formatted = self.format_amount(transaction.amount);
        let color_prefix = if self.use_colors {
            self.get_transaction_color(transaction)
        } else {
            String::new()
        };

        format!(
            "{}{}Transaction: {} → {} | Amount: {} | Token: {} | Block: {}",
            color_prefix,
            timestamp,
            self.truncate_address(&transaction.sender),
            self.truncate_address(&transaction.recipient),
            amount_formatted,
            transaction.token_type,
            transaction.block_number
        )
    }

    pub fn format_alert(&self, alert: &Alert) -> String {
        let timestamp = if self.show_timestamps {
            let now = Utc::now();
            format!("[{}] ", now.format("%Y-%m-%d %H:%M:%S"))
        } else {
            String::new()
        };

        let alert_type = match alert {
            Alert::LowBalance { .. } => "LOW_BALANCE",
            Alert::LargeTransfer { .. } => "LARGE_TRANSFER",
            Alert::SuspiciousActivity { .. } => "SUSPICIOUS_ACTIVITY",
            Alert::NetworkError { .. } => "NETWORK_ERROR",
        };

        let color_prefix = if self.use_colors {
            "\x1b[31m" // 红色
        } else {
            String::new()
        };

        let color_suffix = if self.use_colors {
            "\x1b[0m"
        } else {
            String::new()
        };

        let message = match alert {
            Alert::LowBalance { address, balance, threshold } => {
                format!("Low balance alert for {}: {} (threshold: {})", 
                    self.truncate_address(address), 
                    self.format_amount(*balance), 
                    self.format_amount(*threshold))
            },
            Alert::LargeTransfer { sender, recipient, amount, transaction_id } => {
                format!("Large transfer detected: {} → {} | Amount: {} | TX: {}", 
                    self.truncate_address(sender), 
                    self.truncate_address(recipient), 
                    self.format_amount(*amount), 
                    transaction_id)
            },
            Alert::SuspiciousActivity { address, activity_type, description } => {
                format!("Suspicious activity detected for {}: {} - {}", 
                    self.truncate_address(address), 
                    activity_type, 
                    description)
            },
            Alert::NetworkError { error } => {
                format!("Network error: {}", error)
            },
        };

        format!(
            "{}{}ALERT [{}]: {}{}",
            color_prefix,
            timestamp,
            alert_type,
            message,
            color_suffix
        )
    }

    pub fn format_balance_summary(&self, balances: &HashMap<String, u64>) -> String {
        let mut summary = String::from("=== Balance Summary ===\n");
        
        for (address, balance) in balances {
            summary.push_str(&format!(
                "{}: {}\n",
                self.truncate_address(address),
                self.format_amount(*balance)
            ));
        }
        
        summary
    }

    pub fn format_transaction_history(&self, transactions: &[Transaction]) -> String {
        let mut history = String::from("=== Recent Transactions ===\n");
        
        for (i, tx) in transactions.iter().take(10).enumerate() {
            history.push_str(&format!(
                "{}. {}\n",
                i + 1,
                self.format_transaction(tx)
            ));
        }
        
        history
    }

    fn format_amount(&self, amount: u64) -> String {
        // SUI代币通常有9位小数
        let sui_amount = amount as f64 / 1_000_000_000.0;
        format!("{:.9} SUI", sui_amount)
    }

    fn truncate_address(&self, address: &str) -> String {
        if address.len() > 10 {
            format!("{}...{}", &address[..6], &address[address.len()-4..])
        } else {
            address.to_string()
        }
    }

    fn get_transaction_color(&self, transaction: &Transaction) -> String {
        if transaction.amount > 1_000_000_000 { // 大于1 SUI
            "\x1b[33m" // 黄色
        } else if transaction.amount > 100_000_000 { // 大于0.1 SUI
            "\x1b[32m" // 绿色
        } else {
            "\x1b[37m" // 白色
        }.to_string()
    }
}
```

### 2.4 Phase 4: 完善和集成 (4小时)

#### 步骤9: 实现主应用逻辑
```rust
// src/lib.rs - 继续实现
use crate::config::Config;
use crate::sui_client::SuiClient;
use crate::event_monitor::EventMonitor;
use crate::transaction_processor::TransactionProcessor;
use crate::alert_system::AlertSystem;
use crate::output_formatter::OutputFormatter;
use tokio::sync::mpsc;

impl TokenTransferTracker {
    pub async fn new(config: Config) -> TrackerResult<Self> {
        let sui_client = SuiClient::new(&config.network.rpc_url).await?;
        let (event_monitor, event_receiver) = EventMonitor::new(
            std::sync::Arc::new(sui_client.clone()),
            Duration::from_secs(config.monitoring.poll_interval_seconds),
        ).await;

        let transaction_processor = TransactionProcessor::new();
        let (alert_system, alert_receiver) = AlertSystem::new(config.alerts.large_transfer_threshold);
        let output_formatter = OutputFormatter::new(config.output.use_colors, config.output.show_timestamps);

        let mut monitored_addresses = HashMap::new();
        for address in &config.addresses.monitored {
            monitored_addresses.insert(address.clone(), AddressInfo {
                balance: 0,
                last_checked: 0,
                alert_threshold: config.alerts.low_balance_threshold,
                transactions: Vec::new(),
            });
        }

        Ok(Self {
            config,
            sui_client,
            event_monitor,
            monitored_addresses: RwLock::new(monitored_addresses),
        })
    }

    pub async fn start_monitoring(&mut self) -> TrackerResult<()> {
        println!("Starting SUI Token Transfer Tracker...");
        println!("Monitoring {} addresses", self.config.addresses.monitored.len());

        // 启动事件监控
        let monitor_handle = {
            let event_monitor = self.event_monitor.clone();
            tokio::spawn(async move {
                event_monitor.start_monitoring().await;
            })
        };

        // 启动主处理循环
        self.processing_loop().await?;

        monitor_handle.await?;
        Ok(())
    }

    async fn processing_loop(&mut self) -> TrackerResult<()> {
        let mut event_receiver = self.event_monitor.get_event_receiver();
        let mut alert_receiver = self.alert_system.get_alert_receiver();

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    self.process_event(event).await?;
                }
                Some(alert) = alert_receiver.recv() => {
                    self.handle_alert(alert).await?;
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    // 定期清理和状态更新
                    self.maintenance_tasks().await?;
                }
            }
        }
    }

    async fn process_event(&self, event: TransferEvent) -> TrackerResult<()> {
        // 处理转移事件
        let processed = self.transaction_processor.process_transfer_event(event).await?;

        // 检查警报
        self.alert_system.check_large_transfer(&processed.transaction).await?;
        
        // 检查余额警报
        let sender_balance = self.transaction_processor.get_address_balance(&processed.transaction.sender).await;
        let receiver_balance = self.transaction_processor.get_address_balance(&processed.transaction.recipient).await;
        
        self.alert_system.check_balance_alert(&processed.transaction.sender, sender_balance).await?;
        self.alert_system.check_balance_alert(&processed.transaction.recipient, receiver_balance).await?;

        // 输出交易信息
        let formatted = self.output_formatter.format_transaction(&processed.transaction);
        println!("{}", formatted);

        Ok(())
    }

    async fn handle_alert(&self, alert: Alert) -> TrackerResult<()> {
        let formatted = self.output_formatter.format_alert(&alert);
        eprintln!("{}", formatted);
        Ok(())
    }

    async fn maintenance_tasks(&self) -> TrackerResult<()> {
        // 清理过期交易记录
        self.transaction_processor.cleanup_old_transactions(86400).await?; // 24小时
        
        // 定期输出余额摘要
        let balances = self.transaction_processor.get_all_balances().await;
        let summary = self.output_formatter.format_balance_summary(&balances);
        println!("{}", summary);

        Ok(())
    }
}
```

#### 步骤10: 实现主入口点
```rust
// src/main.rs
use clap::{App, Arg};
use sui_token_tracker::{TokenTransferTracker, Config, error::TrackerResult};

#[tokio::main]
async fn main() -> TrackerResult<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let matches = App::new("SUI Token Transfer Tracker")
        .version("0.1.0")
        .about("Real-time monitoring of SUI token transfers")
        .arg(Arg::with_name("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Path to configuration file")
            .takes_value(true))
        .arg(Arg::with_name("address")
            .short('a')
            .long("address")
            .value_name("ADDRESS")
            .help("SUI address to monitor")
            .takes_value(true)
            .multiple(true))
        .arg(Arg::with_name("poll-interval")
            .short('i')
            .long("poll-interval")
            .value_name("SECONDS")
            .help("Polling interval in seconds")
            .takes_value(true))
        .arg(Arg::with_name("threshold")
            .short('t')
            .long("threshold")
            .value_name("AMOUNT")
            .help("Low balance threshold in SUI")
            .takes_value(true))
        .get_matches();

    // 加载配置
    let config = Config::load(matches.value_of("config"))?;

    // 应用命令行参数覆盖
    let mut config = config;
    
    if let Some(addresses) = matches.values_of("address") {
        config.addresses.monitored = addresses.map(|s| s.to_string()).collect();
    }
    
    if let Some(interval) = matches.value_of("poll-interval") {
        config.monitoring.poll_interval_seconds = interval.parse()
            .map_err(|_| anyhow::anyhow!("Invalid poll interval"))?;
    }
    
    if let Some(threshold) = matches.value_of("threshold") {
        config.alerts.low_balance_threshold = threshold.parse()
            .map_err(|_| anyhow::anyhow!("Invalid threshold"))?;
    }

    // 创建并启动跟踪器
    let mut tracker = TokenTransferTracker::new(config).await?;
    tracker.start_monitoring().await?;

    Ok(())
}
```

---

## 3. 配置管理

### 3.1 配置文件结构 (config.toml)
```toml
[network]
rpc_url = "https://fullnode.mainnet.sui.io:443"
websocket_url = "wss://fullnode.mainnet.sui.io"
timeout_seconds = 30

[monitoring]
poll_interval_seconds = 10
max_history_records = 1000
batch_size = 50
cleanup_interval_hours = 24

[addresses]
# 要监控的地址列表
monitored = [
    "0x1234567890abcdef1234567890abcdef12345678",
    "0xabcdef1234567890abcdef1234567890abcdef12"
]

[alerts]
low_balance_threshold = 1000000000  # 1 SUI
large_transfer_threshold = 10000000000  # 10 SUI
enable_console_alerts = true
enable_file_alerts = false
alert_file_path = "alerts.log"

[output]
use_colors = true
show_timestamps = true
max_recent_transactions = 10
balance_summary_interval = 300  # 5分钟

[logging]
level = "info"
file_path = "tracker.log"
max_file_size_mb = 10
rotate_files = 5
```

### 3.2 配置模块实现
```rust
// src/config.rs
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
                    .map_err(|e| TrackerError::ConfigurationError(
                        format!("Failed to read config file: {}", e)
                    ))?;
                
                toml::from_str(&content)
                    .map_err(|e| TrackerError::ConfigurationError(
                        format!("Failed to parse config file: {}", e)
                    ))
            }
            None => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &Path) -> TrackerResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| TrackerError::ConfigurationError(
                format!("Failed to serialize config: {}", e)
            ))?;
        
        std::fs::write(path, content)
            .map_err(|e| TrackerError::ConfigurationError(
                format!("Failed to write config file: {}", e)
            ))?;
        
        Ok(())
    }

    pub fn validate(&self) -> TrackerResult<()> {
        if self.network.rpc_url.is_empty() {
            return Err(TrackerError::ConfigurationError(
                "RPC URL cannot be empty".into()
            ));
        }

        if self.monitoring.poll_interval_seconds == 0 {
            return Err(TrackerError::ConfigurationError(
                "Poll interval must be greater than 0".into()
            ));
        }

        if self.monitoring.max_history_records == 0 {
            return Err(TrackerError::ConfigurationError(
                "Max history records must be greater than 0".into()
            ));
        }

        Ok(())
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
```

---

## 4. 错误处理

### 4.1 错误类型定义
```rust
// src/error.rs
use thiserror::Error;

pub type TrackerResult<T> = Result<T, TrackerError>;

#[derive(Error, Debug)]
pub enum TrackerError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Sui client error: {0}")]
    SuiClientError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
}

impl TrackerError {
    pub fn network_error(msg: impl Into<String>) -> Self {
        TrackerError::NetworkError(reqwest::Error::from(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            msg.into(),
        )))
    }

    pub fn sui_client_error(msg: impl Into<String>) -> Self {
        TrackerError::SuiClientError(msg.into())
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        TrackerError::ParseError(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        TrackerError::ConfigurationError(msg.into())
    }
}
```

### 4.2 重试机制
```rust
// src/utils.rs
use tokio::time::{sleep, Duration};
use crate::error::{TrackerError, TrackerResult};

pub async fn retry_operation<T, F, Fut>(
    mut operation: F,
    max_retries: u32,
    base_delay_ms: u64,
) -> TrackerResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = TrackerResult<T>>,
{
    let mut retries = 0;
    
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                retries += 1;
                let delay_ms = base_delay_ms * 2u64.pow(retries - 1);
                log::warn!("Operation failed (attempt {}/{}): {}, retrying in {}ms", 
                    retries, max_retries, e, delay_ms);
                sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }
            Err(e) => {
                log::error!("Operation failed after {} attempts: {}", max_retries, e);
                return Err(e);
            }
        }
    }
}

pub async fn with_timeout<T, Fut>(future: Fut, timeout_secs: u64) -> TrackerResult<T>
where
    Fut: std::future::Future<Output = TrackerResult<T>>,
{
    match tokio::time::timeout(Duration::from_secs(timeout_secs), future).await {
        Ok(result) => result,
        Err(_) => Err(TrackerError::TimeoutError(
            format!("Operation timed out after {} seconds", timeout_secs)
        )),
    }
}
```

---

## 5. 测试和验证

### 5.1 单元测试
```rust
// tests/unit_tests.rs
use sui_token_tracker::{
    TransactionProcessor, AlertSystem, OutputFormatter, TransferEvent, Transaction
};

#[tokio::test]
async fn test_transaction_processor() {
    let processor = TransactionProcessor::new();
    
    let event = TransferEvent {
        transaction_id: "0x123".to_string(),
        package_id: "0x456".to_string(),
        transaction_module: "test".to_string(),
        sender: "0xsender".to_string(),
        recipient: "0xrecipient".to_string(),
        amount: 1000000000,
        token_type: "0x2::sui::SUI".to_string(),
        timestamp: 1634567890,
        block_number: 12345,
    };

    let result = processor.process_transfer_event(event).await.unwrap();
    assert_eq!(result.transaction.amount, 1000000000);
    assert_eq!(result.sender_balance_change, -1000000000);
    assert_eq!(result.receiver_balance_change, 1000000000);
}

#[tokio::test]
async fn test_alert_system() {
    let (alert_system, mut alert_receiver) = AlertSystem::new(5000000000); // 5 SUI threshold
    
    // 测试余额警报
    alert_system.set_threshold("0xtest".to_string(), 1000000000).await;
    alert_system.check_balance_alert("0xtest", 500000000).await.unwrap();
    
    if let Some(alert) = alert_receiver.recv().await {
        match alert {
            sui_token_tracker::Alert::LowBalance { address, balance, threshold } => {
                assert_eq!(address, "0xtest");
                assert_eq!(balance, 500000000);
                assert_eq!(threshold, 1000000000);
            }
            _ => panic!("Expected LowBalance alert"),
        }
    }
}

#[test]
fn test_output_formatter() {
    let formatter = OutputFormatter::new(false, true);
    
    let transaction = Transaction {
        id: "0x123".to_string(),
        sender: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        recipient: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
        amount: 1000000000,
        token_type: "0x2::sui::SUI".to_string(),
        timestamp: 1634567890,
        block_number: 12345,
    };

    let formatted = formatter.format_transaction(&transaction);
    assert!(formatted.contains("0x123456...45678"));
    assert!(formatted.contains("0xabcded...cdef12"));
    assert!(formatted.contains("1.000000000 SUI"));
}
```

### 5.2 集成测试
```rust
// tests/integration_tests.rs
use sui_token_tracker::{TokenTransferTracker, Config};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_full_monitoring_cycle() {
    // 创建测试配置
    let config = Config {
        network: sui_token_tracker::NetworkConfig {
            rpc_url: "https://fullnode.testnet.sui.io:443".to_string(),
            websocket_url: "wss://fullnode.testnet.sui.io".to_string(),
            timeout_seconds: 10,
        },
        monitoring: sui_token_tracker::MonitoringConfig {
            poll_interval_seconds: 1,
            max_history_records: 100,
            batch_size: 10,
            cleanup_interval_hours: 1,
        },
        addresses: sui_token_tracker::AddressConfig {
            monitored: vec![
                "0x1234567890abcdef1234567890abcdef12345678".to_string()
            ],
        },
        alerts: sui_token_tracker::AlertConfig {
            low_balance_threshold: 1000000000,
            large_transfer_threshold: 5000000000,
            enable_console_alerts: true,
            enable_file_alerts: false,
            alert_file_path: "test_alerts.log".to_string(),
        },
        output: sui_token_tracker::OutputConfig {
            use_colors: false,
            show_timestamps: true,
            max_recent_transactions: 5,
            balance_summary_interval: 60,
        },
        logging: sui_token_tracker::LoggingConfig {
            level: "debug".to_string(),
            file_path: "test_tracker.log".to_string(),
            max_file_size_mb: 1,
            rotate_files: 1,
        },
    };

    // 创建跟踪器
    let mut tracker = TokenTransferTracker::new(config).await.unwrap();
    
    // 启动监控（短时间内测试）
    let monitor_handle = tokio::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        // 正常情况下这里会继续运行，测试中我们手动停止
    });

    // 等待监控启动
    sleep(Duration::from_secs(2)).await;
    
    // 验证跟踪器状态
    assert!(!monitor_handle.is_finished());
    
    // 清理
    monitor_handle.abort();
    
    // 清理测试文件
    std::fs::remove_file("test_alerts.log").ok();
    std::fs::remove_file("test_tracker.log").ok();
}
```

---

## 6. 部署指南

### 6.1 本地开发部署
```bash
# 1. 克隆项目
git clone <repository-url>
cd sui-token-transfer-tracker

# 2. 安装依赖
cargo build

# 3. 运行测试
cargo test

# 4. 创建配置文件
cp config/example.toml config/config.toml
# 编辑配置文件

# 5. 运行应用
cargo run -- --config config/config.toml

# 或者使用命令行参数
cargo run -- --address 0x1234... --poll-interval 5 --threshold 1000000000
```

### 6.2 生产环境部署
```bash
# 1. 编译发布版本
cargo build --release

# 2. 创建系统服务 (Linux)
sudo tee /etc/systemd/system/sui-tracker.service > /dev/null <<EOF
[Unit]
Description=SUI Token Transfer Tracker
After=network.target

[Service]
Type=simple
User=sui-tracker
WorkingDirectory=/opt/sui-tracker
ExecStart=/opt/sui-tracker/target/release/sui-token-transfer-tracker --config /opt/sui-tracker/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 3. 启动服务
sudo systemctl daemon-reload
sudo systemctl enable sui-tracker
sudo systemctl start sui-tracker

# 4. 查看状态
sudo systemctl status sui-tracker
sudo journalctl -u sui-tracker -f
```

### 6.3 Docker部署
```dockerfile
# Dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/sui-token-transfer-tracker .
COPY config/config.toml .

EXPOSE 8080

CMD ["./sui-token-transfer-tracker", "--config", "config.toml"]
```

```bash
# 构建和运行Docker镜像
docker build -t sui-token-tracker .
docker run -d --name sui-tracker -v $(pwd)/config:/app/config sui-token-tracker
```

---

## 7. 使用示例

### 7.1 基本使用
```bash
# 监控单个地址
cargo run -- --address 0x1234567890abcdef1234567890abcdef12345678

# 监控多个地址
cargo run -- --address 0x1234... --address 0x5678...

# 自定义轮询间隔
cargo run -- --address 0x1234... --poll-interval 5

# 设置余额警报阈值
cargo run -- --address 0x1234... --threshold 500000000
```

### 7.2 使用配置文件
```bash
# 使用自定义配置文件
cargo run -- --config my-config.toml

# 生成默认配置文件
cargo run -- --generate-config > default-config.toml
```

### 7.3 输出示例
```
[2023-10-20 14:30:15] Transaction: 0x1234...6789 → 0xabcd...ef12 | Amount: 1.500000000 SUI | Token: 0x2::sui::SUI | Block: 987654
[2023-10-20 14:30:20] ALERT [LOW_BALANCE]: Low balance alert for 0x1234...6789: 0.500000000 SUI (threshold: 1.000000000 SUI)
[2023-10-20 14:30:25] ALERT [LARGE_TRANSFER]: Large transfer detected: 0xabcd...ef12 → 0x5678...9012 | Amount: 10.000000000 SUI | TX: 0x9999...8888

=== Balance Summary ===
0x1234...6789: 0.500000000 SUI
0xabcd...ef12: 15.500000000 SUI
0x5678...9012: 10.000000000 SUI
```

---

## 8. 性能优化

### 8.1 性能监控指标
- 事件查询延迟
- 内存使用量
- 交易处理速度
- 网络请求成功率

### 8.2 优化策略
- **批量查询**: 减少RPC调用次数
- **连接池**: 复用HTTP连接
- **内存限制**: 限制历史记录数量
- **缓存策略**: 缓存频繁查询的数据

### 8.3 监控和日志
```rust
// 添加性能监控
use std::time::Instant;

pub struct PerformanceMonitor {
    query_times: Vec<Duration>,
    memory_usage: Vec<u64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            query_times: Vec::new(),
            memory_usage: Vec::new(),
        }
    }

    pub fn record_query_time(&mut self, duration: Duration) {
        self.query_times.push(duration);
        if self.query_times.len() > 1000 {
            self.query_times.remove(0);
        }
    }

    pub fn get_average_query_time(&self) -> Duration {
        if self.query_times.is_empty() {
            return Duration::from_millis(0);
        }
        let total: Duration = self.query_times.iter().sum();
        total / self.query_times.len() as u32
    }

    pub fn log_stats(&self) {
        log::info!("Performance stats:");
        log::info!("  Average query time: {:?}", self.get_average_query_time());
        log::info!("  Total queries: {}", self.query_times.len());
    }
}
```

---

## 9. 扩展功能

### 9.1 数据导出
```rust
pub trait DataExporter {
    fn export_transactions(&self, transactions: &[Transaction], format: ExportFormat) -> Result<String, ExportError>;
    fn export_balances(&self, balances: &HashMap<String, u64>, format: ExportFormat) -> Result<String, ExportError>;
}

pub enum ExportFormat {
    Csv,
    Json,
    Toml,
}
```

### 9.2 通知系统
```rust
pub trait NotificationSystem {
    async fn send_notification(&self, alert: &Alert) -> Result<(), NotificationError>;
    async fn send_summary(&self, summary: &str) -> Result<(), NotificationError>;
}

pub struct EmailNotifier {
    smtp_server: String,
    sender_email: String,
    recipients: Vec<String>,
}

pub struct DiscordNotifier {
    webhook_url: String,
}
```

### 9.3 Web界面
```rust
// 简单的Web服务器提供状态查看
use warp::Filter;

pub async fn start_web_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let status_route = warp::path("status")
        .map(|| warp::reply::json(&serde_json::json!({"status": "running"})));

    let balances_route = warp::path("balances")
        .map(|| warp::reply::json(&serde_json::json!({"balances": []})));

    let routes = status_route.or(balances_route);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;

    Ok(())
}
```

---

## 10. 总结

这个技术需求文档提供了SUI Token Transfer Tracker项目的完整实现方案，包括：

1. **完整的项目架构设计**
2. **详细的实现步骤和代码示例**
3. **配置管理和错误处理**
4. **测试和部署指南**
5. **性能优化和扩展功能**

项目采用Rust语言开发，利用Sui SDK进行区块链交互，具有高性能、可靠性和可扩展性的特点。通过模块化设计，便于后续功能扩展和维护。

整个实现方案遵循最佳实践，包括异步编程、错误处理、配置管理等方面，为黑客松项目提供了一个完整的技术实现路径。