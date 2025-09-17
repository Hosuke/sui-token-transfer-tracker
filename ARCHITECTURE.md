# SUI Token Transfer Tracker - 技术架构设计

## 系统架构图

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

## 核心组件设计

### 1. 主应用程序结构

```rust
// src/main.rs
use clap::{App, Arg};
use sui_token_tracker::{TokenTransferTracker, Config};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("SUI Token Transfer Tracker")
        .version("1.0")
        .about("Monitor SUI token transfers in real-time")
        .arg(Arg::with_name("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .get_matches();

    let config = Config::load(matches.value_of("config"))?;
    let mut tracker = TokenTransferTracker::new(config).await?;
    
    tracker.start_monitoring().await
}
```

### 2. 核心跟踪器模块

```rust
// src/lib.rs
pub mod config;
pub mod sui_client;
pub mod event_monitor;
pub mod alert_system;
pub mod transaction_processor;
pub mod output_formatter;

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
```

### 3. SUI客户端封装

```rust
// src/sui_client.rs
use sui_sdk::SuiClient as SuiSdkClient;
use sui_sdk::rpc_types::{SuiEvent, EventFilter};

pub struct SuiClient {
    client: SuiSdkClient,
    network_url: String,
}

impl SuiClient {
    pub async fn new(network_url: &str) -> Result<Self> {
        let client = SuiSdkClient::new(network_url).await?;
        Ok(Self {
            client,
            network_url: network_url.to_string(),
        })
    }

    pub async fn query_transfer_events(
        &self,
        address: &str,
        limit: u32,
    ) -> Result<Vec<SuiEvent>> {
        let filter = EventFilter::Sender(address.parse()?);
        let events = self.client.query_events(filter, limit).await?;
        Ok(events)
    }

    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let balance = self.client.get_balance(address).await?;
        Ok(balance)
    }

    pub async fn subscribe_to_events(&self, address: &str) -> Result<EventStream> {
        let stream = self.client.subscribe_events(EventFilter::Sender(address.parse()?)).await?;
        Ok(stream)
    }
}
```

### 4. 事件监控器

```rust
// src/event_monitor.rs
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::collections::HashSet;

pub struct EventMonitor {
    sui_client: Arc<SuiClient>,
    poll_interval: Duration,
    addresses: RwLock<HashSet<String>>,
    event_sender: mpsc::UnboundedSender<TransferEvent>,
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

    pub async fn add_address(&self, address: String) -> Result<()> {
        self.addresses.write().await.insert(address);
        Ok(())
    }

    pub async fn start_monitoring(&self) {
        let mut interval = interval(self.poll_interval);
        loop {
            interval.tick().await;
            self.check_new_events().await;
        }
    }

    async fn check_new_events(&self) {
        let addresses = self.addresses.read().await;
        for address in addresses.iter() {
            if let Ok(events) = self.sui_client.query_transfer_events(address, 10).await {
                for event in events {
                    if let Ok(transfer_event) = self.parse_transfer_event(event) {
                        let _ = self.event_sender.send(transfer_event);
                    }
                }
            }
        }
    }
}
```

### 5. 交易处理器

```rust
// src/transaction_processor.rs
use std::collections::HashMap;

pub struct TransactionProcessor {
    address_balances: RwLock<HashMap<String, u64>>,
    transaction_history: RwLock<HashMap<String, Vec<Transaction>>>,
}

impl TransactionProcessor {
    pub fn new() -> Self {
        Self {
            address_balances: RwLock::new(HashMap::new()),
            transaction_history: RwLock::new(HashMap::new()),
        }
    }

    pub async fn process_transfer_event(&self, event: TransferEvent) -> Result<ProcessedTransaction> {
        let mut balances = self.address_balances.write().await;
        let mut history = self.transaction_history.write().await;

        // Update sender balance
        let sender_balance = balances.entry(event.sender.clone()).or_insert(0);
        *sender_balance = sender_balance.saturating_sub(event.amount);

        // Update receiver balance
        let receiver_balance = balances.entry(event.recipient.clone()).or_insert(0);
        *receiver_balance = receiver_balance.saturating_add(event.amount);

        // Create transaction record
        let transaction = Transaction {
            id: event.transaction_id.clone(),
            sender: event.sender.clone(),
            recipient: event.recipient.clone(),
            amount: event.amount,
            token_type: event.token_type,
            timestamp: event.timestamp,
            block_number: event.block_number,
        };

        // Add to history
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

    pub async fn get_address_history(&self, address: &str, limit: u32) -> Vec<Transaction> {
        let history = self.transaction_history.read().await;
        history.get(address)
            .map(|transactions| transactions.iter().take(limit as usize).cloned().collect())
            .unwrap_or_default()
    }
}
```

### 6. 警报系统

```rust
// src/alert_system.rs
use tokio::sync::mpsc;
use std::collections::HashMap;

pub struct AlertSystem {
    thresholds: RwLock<HashMap<String, u64>>,
    alert_sender: mpsc::UnboundedSender<Alert>,
}

pub enum Alert {
    LowBalance { address: String, balance: u64, threshold: u64 },
    LargeTransfer { sender: String, recipient: String, amount: u64 },
    SuspiciousActivity { address: String, activity_type: String },
}

impl AlertSystem {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Alert>) {
        let (alert_sender, alert_receiver) = mpsc::unbounded_channel();
        let system = Self {
            thresholds: RwLock::new(HashMap::new()),
            alert_sender,
        };
        (system, alert_receiver)
    }

    pub async fn set_threshold(&self, address: String, threshold: u64) {
        self.thresholds.write().await.insert(address, threshold);
    }

    pub async fn check_balance_alert(&self, address: &str, balance: u64) {
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
    }

    pub async fn check_large_transfer(&self, amount: u64, threshold: u64) {
        if amount > threshold {
            // Need to get transaction details for full alert
            // This would be called from transaction processor
        }
    }
}
```

## 数据流设计

### 事件处理流程
```
Sui Network → Event Query → Event Parsing → Transaction Processing → Alert Check → Output
     ↓              ↓             ↓               ↓                ↓            ↓
  WebSocket   RPC Call     Data Validation   Balance Update   Threshold Check   CLI/UI
```

### 监控循环
```rust
async fn monitoring_loop(tracker: &mut TokenTransferTracker) -> Result<()> {
    let mut event_receiver = tracker.event_monitor.get_event_receiver();
    let mut alert_receiver = tracker.alert_system.get_alert_receiver();

    loop {
        tokio::select! {
            Some(event) = event_receiver.recv() => {
                tracker.process_event(event).await?;
            }
            Some(alert) = alert_receiver.recv() => {
                tracker.handle_alert(alert).await?;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // Periodic tasks (cleanup, stats, etc.)
            }
        }
    }
}
```

## 错误处理策略

### 网络错误处理
```rust
pub enum TrackerError {
    NetworkError(reqwest::Error),
    SuiClientError(sui_sdk::error::Error),
    ParseError(serde_json::Error),
    ConfigurationError(String),
}

impl std::fmt::Display for TrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackerError::NetworkError(e) => write!(f, "Network error: {}", e),
            TrackerError::SuiClientError(e) => write!(f, "Sui client error: {}", e),
            TrackerError::ParseError(e) => write!(f, "Parse error: {}", e),
            TrackerError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for TrackerError {}
```

### 重试机制
```rust
pub async fn retry_operation<T, F, Fut>(mut operation: F, max_retries: u32) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut retries = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                retries += 1;
                tokio::time::sleep(Duration::from_secs(2u64.pow(retries))).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## 性能优化策略

### 1. 批量查询
- 使用Sui SDK的批量查询功能
- 缓存频繁查询的地址信息
- 并行处理多个地址的事件

### 2. 内存管理
- 限制历史记录数量
- 定期清理过期数据
- 使用高效的内存结构

### 3. 网络优化
- 连接池复用
- 请求超时控制
- 压缩传输数据

## 配置管理

### 配置文件结构 (config.toml)
```toml
[network]
rpc_url = "https://fullnode.mainnet.sui.io:443"
websocket_url = "wss://fullnode.mainnet.sui.io"

[monitoring]
poll_interval_seconds = 10
max_history_records = 1000
batch_size = 50

[addresses]
# 可以添加要监控的地址列表
monitored = [
    "0x1234567890abcdef1234567890abcdef12345678",
    "0xabcdef1234567890abcdef1234567890abcdef12"
]

[alerts]
low_balance_threshold = 1000000000
large_transfer_threshold = 10000000000
enable_email_alerts = false
```

这个架构设计提供了完整的技术实现方案，支持项目的主要功能需求，并考虑了可扩展性、性能和错误处理。