use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::event_monitor::TransferEvent;
use crate::error::{TrackerError, TrackerResult};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct TransactionProcessor {
    address_balances: RwLock<HashMap<String, u64>>,
    transaction_history: RwLock<HashMap<String, Vec<Transaction>>>,
    address_stats: RwLock<HashMap<String, AddressStats>>,
    config: ProcessorConfig,
}

#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub max_history_records: u32,
    pub cleanup_interval_hours: u64,
    pub enable_detailed_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub token_type: String,
    pub timestamp: u64,
    pub block_number: u64,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failed,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    pub transaction: Transaction,
    pub sender_balance_change: i64,
    pub receiver_balance_change: i64,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressStats {
    pub total_transactions: u64,
    pub total_sent: u64,
    pub total_received: u64,
    pub first_transaction: Option<u64>,
    pub last_transaction: Option<u64>,
    pub average_transaction_amount: u64,
    pub largest_transaction: u64,
    pub smallest_transaction: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistory {
    pub address: String,
    pub history: Vec<BalanceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub timestamp: u64,
    pub balance: u64,
    pub transaction_id: Option<String>,
}

impl TransactionProcessor {
    pub fn new() -> Self {
        Self::with_config(ProcessorConfig {
            max_history_records: 1000,
            cleanup_interval_hours: 24,
            enable_detailed_stats: true,
        })
    }

    pub fn with_config(config: ProcessorConfig) -> Self {
        Self {
            address_balances: RwLock::new(HashMap::new()),
            transaction_history: RwLock::new(HashMap::new()),
            address_stats: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub async fn process_transfer_event(&self, event: TransferEvent) -> TrackerResult<ProcessedTransaction> {
        let start_time = SystemTime::now();
        let processing_start = start_time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        let mut balances = self.address_balances.write().await;
        let mut history = self.transaction_history.write().await;
        let mut stats = self.address_stats.write().await;

        // 更新发送方余额
        let sender_balance = balances.entry(event.sender.clone()).or_insert(0);
        *sender_balance = sender_balance.saturating_sub(event.amount);

        // 更新接收方余额
        let receiver_balance = balances.entry(event.recipient.clone()).or_insert(0);
        *receiver_balance = receiver_balance.saturating_add(event.amount);

        // 创建交易记录
        let transaction = Transaction {
            id: event.transaction_id.clone(),
            sender: event.sender.clone(),
            recipient: event.recipient.clone(),
            amount: event.amount,
            token_type: event.token_type,
            timestamp: event.timestamp,
            block_number: event.block_number,
            gas_used: None, // 可以从交易详情中获取
            gas_price: None, // 可以从交易详情中获取
            status: TransactionStatus::Success,
        };

        // 添加到历史记录
        history.entry(event.sender.clone())
            .or_insert_with(Vec::new)
            .push(transaction.clone());
        
        history.entry(event.recipient.clone())
            .or_insert_with(Vec::new)
            .push(transaction.clone());

        // 更新统计信息
        self.update_address_stats(&mut stats, &event.sender, &event.recipient, &transaction).await?;

        // 处理历史记录限制
        self.enforce_history_limits(&mut history).await;

        let processing_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let processing_time = processing_end.saturating_sub(processing_start);

        Ok(ProcessedTransaction {
            transaction,
            sender_balance_change: -(event.amount as i64),
            receiver_balance_change: event.amount as i64,
            processing_time_ms: processing_time,
        })
    }

    async fn update_address_stats(
        &self,
        stats: &mut HashMap<String, AddressStats>,
        sender: &str,
        recipient: &str,
        transaction: &Transaction,
    ) -> TrackerResult<()> {
        // 更新发送方统计
        let sender_stats = stats.entry(sender.to_string()).or_insert(AddressStats {
            total_transactions: 0,
            total_sent: 0,
            total_received: 0,
            first_transaction: None,
            last_transaction: None,
            average_transaction_amount: 0,
            largest_transaction: 0,
            smallest_transaction: u64::MAX,
        });

        sender_stats.total_transactions += 1;
        sender_stats.total_sent += transaction.amount;
        sender_stats.largest_transaction = sender_stats.largest_transaction.max(transaction.amount);
        sender_stats.smallest_transaction = sender_stats.smallest_transaction.min(transaction.amount);
        
        if sender_stats.first_transaction.is_none() || transaction.timestamp < sender_stats.first_transaction.unwrap() {
            sender_stats.first_transaction = Some(transaction.timestamp);
        }
        if sender_stats.last_transaction.is_none() || transaction.timestamp > sender_stats.last_transaction.unwrap() {
            sender_stats.last_transaction = Some(transaction.timestamp);
        }

        // 更新接收方统计
        let receiver_stats = stats.entry(recipient.to_string()).or_insert(AddressStats {
            total_transactions: 0,
            total_sent: 0,
            total_received: 0,
            first_transaction: None,
            last_transaction: None,
            average_transaction_amount: 0,
            largest_transaction: 0,
            smallest_transaction: u64::MAX,
        });

        receiver_stats.total_transactions += 1;
        receiver_stats.total_received += transaction.amount;
        receiver_stats.largest_transaction = receiver_stats.largest_transaction.max(transaction.amount);
        receiver_stats.smallest_transaction = receiver_stats.smallest_transaction.min(transaction.amount);
        
        if receiver_stats.first_transaction.is_none() || transaction.timestamp < receiver_stats.first_transaction.unwrap() {
            receiver_stats.first_transaction = Some(transaction.timestamp);
        }
        if receiver_stats.last_transaction.is_none() || transaction.timestamp > receiver_stats.last_transaction.unwrap() {
            receiver_stats.last_transaction = Some(transaction.timestamp);
        }

        // 计算平均交易金额
        for (_, address_stats) in stats.iter_mut() {
            if address_stats.total_transactions > 0 {
                let total_amount = address_stats.total_sent + address_stats.total_received;
                address_stats.average_transaction_amount = total_amount / address_stats.total_transactions;
            }
        }

        Ok(())
    }

    async fn enforce_history_limits(&self, history: &mut HashMap<String, Vec<Transaction>>) {
        let max_records = self.config.max_history_records as usize;
        
        for (_, transactions) in history.iter_mut() {
            if transactions.len() > max_records {
                transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                transactions.truncate(max_records);
            }
        }
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
        balances.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    pub async fn get_address_stats(&self, address: &str) -> Option<AddressStats> {
        let stats = self.address_stats.read().await;
        stats.get(address).cloned()
    }

    pub async fn get_all_stats(&self) -> HashMap<String, AddressStats> {
        let stats = self.address_stats.read().await;
        stats.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub async fn cleanup_old_transactions(&self, max_age_seconds: u64) -> TrackerResult<u64> {
        let current_time = Utc::now().timestamp() as u64;
        let mut history = self.transaction_history.write().await;
        let mut removed_count = 0;

        for (_, transactions) in history.iter_mut() {
            let initial_len = transactions.len();
            transactions.retain(|tx| {
                current_time.saturating_sub(tx.timestamp) <= max_age_seconds
            });
            removed_count += initial_len.saturating_sub(transactions.len());
        }

        log::info!("Cleaned up {} old transaction records", removed_count);
        Ok(removed_count as u64)
    }

    pub async fn get_balance_history(&self, address: &str, limit: u32) -> BalanceHistory {
        let history = self.transaction_history.read().await;
        let mut snapshots = Vec::new();

        if let Some(transactions) = history.get(address) {
            let mut sorted_txs = transactions.clone();
            sorted_txs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            let mut current_balance = 0u64;
            for tx in sorted_txs.iter().take(limit as usize) {
                if tx.sender == address {
                    current_balance = current_balance.saturating_sub(tx.amount);
                } else {
                    current_balance = current_balance.saturating_add(tx.amount);
                }

                snapshots.push(BalanceSnapshot {
                    timestamp: tx.timestamp,
                    balance: current_balance,
                    transaction_id: Some(tx.id.clone()),
                });
            }
        }

        BalanceHistory {
            address: address.to_string(),
            history: snapshots,
        }
    }

    pub async fn get_recent_transactions(&self, limit: u32) -> Vec<Transaction> {
        let history = self.transaction_history.read().await;
        let mut all_transactions: Vec<Transaction> = history
            .values()
            .flat_map(|txs| txs.clone())
            .collect();

        all_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_transactions.into_iter().take(limit as usize).collect()
    }

    pub async fn get_transaction_volume_stats(&self, time_range_hours: u64) -> HashMap<String, u64> {
        let current_time = Utc::now().timestamp() as u64;
        let start_time = current_time.saturating_sub(time_range_hours * 3600);
        
        let history = self.transaction_history.read().await;
        let mut volume_stats = HashMap::new();

        for (_, transactions) in history.iter() {
            for tx in transactions {
                if tx.timestamp >= start_time {
                    *volume_stats.entry(tx.token_type.clone()).or_insert(0) += tx.amount;
                }
            }
        }

        volume_stats
    }

    pub async fn export_data(&self, format: ExportFormat) -> Result<String, TrackerError> {
        match format {
            ExportFormat::Json => {
                let data = serde_json::json!({
                    "balances": *self.address_balances.read().await,
                    "stats": *self.address_stats.read().await,
                    "export_time": Utc::now().to_rfc3339()
                });
                serde_json::to_string_pretty(&data)
                    .map_err(|e| TrackerError::SerializationError(e))
            }
            ExportFormat::Csv => {
                let mut csv = String::new();
                csv.push_str("Address,Balance,Total Transactions,Total Sent,Total Received\n");
                
                let balances = self.address_balances.read().await;
                let stats = self.address_stats.read().await;
                
                for (address, balance) in balances.iter() {
                    if let Some(address_stats) = stats.get(address) {
                        csv.push_str(&format!(
                            "{},{},{},{},{}\n",
                            address,
                            balance,
                            address_stats.total_transactions,
                            address_stats.total_sent,
                            address_stats.total_received
                        ));
                    }
                }
                
                Ok(csv)
            }
        }
    }

    pub async fn get_processor_stats(&self) -> ProcessorStats {
        let balances = self.address_balances.read().await;
        let stats = self.address_stats.read().await;

        let total_transactions: u64 = stats.values().map(|s| s.total_transactions).sum();
        let total_volume: u64 = stats.values().map(|s| s.total_sent + s.total_received).sum();

        ProcessorStats {
            total_addresses: balances.len(),
            total_transactions,
            total_volume,
            config: self.config.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorStats {
    pub total_addresses: usize,
    pub total_transactions: u64,
    pub total_volume: u64,
    pub config: ProcessorConfig,
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_processor_creation() {
        let processor = TransactionProcessor::new();
        let stats = processor.get_processor_stats().await;
        
        assert_eq!(stats.total_addresses, 0);
        assert_eq!(stats.total_transactions, 0);
    }

    #[tokio::test]
    async fn test_process_transfer_event() {
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
            event_type: "transfer".to_string(),
        };

        let result = processor.process_transfer_event(event).await.unwrap();
        assert_eq!(result.transaction.amount, 1000000000);
        assert_eq!(result.sender_balance_change, -1000000000);
        assert_eq!(result.receiver_balance_change, 1000000000);
        assert!(result.processing_time_ms > 0);
    }

    #[tokio::test]
    async fn test_address_balance() {
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
            event_type: "transfer".to_string(),
        };

        processor.process_transfer_event(event).await.unwrap();
        
        assert_eq!(processor.get_address_balance("0xsender").await, 0);
        assert_eq!(processor.get_address_balance("0xrecipient").await, 1000000000);
    }

    #[tokio::test]
    async fn test_address_history() {
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
            event_type: "transfer".to_string(),
        };

        processor.process_transfer_event(event).await.unwrap();
        
        let sender_history = processor.get_address_history("0xsender", 10).await;
        let recipient_history = processor.get_address_history("0xrecipient", 10).await;
        
        assert_eq!(sender_history.len(), 1);
        assert_eq!(recipient_history.len(), 1);
        assert_eq!(sender_history[0].amount, 1000000000);
        assert_eq!(recipient_history[0].amount, 1000000000);
    }

    #[tokio::test]
    async fn test_cleanup_old_transactions() {
        let processor = TransactionProcessor::with_config(ProcessorConfig {
            max_history_records: 10,
            cleanup_interval_hours: 24,
            enable_detailed_stats: true,
        });
        
        // 创建一个旧交易
        let old_event = TransferEvent {
            transaction_id: "0xold".to_string(),
            package_id: "0x456".to_string(),
            transaction_module: "test".to_string(),
            sender: "0xsender".to_string(),
            recipient: "0xrecipient".to_string(),
            amount: 1000000000,
            token_type: "0x2::sui::SUI".to_string(),
            timestamp: 1000000000, // 很旧的时间戳
            block_number: 12345,
            event_type: "transfer".to_string(),
        };

        processor.process_transfer_event(old_event).await.unwrap();
        
        let removed = processor.cleanup_old_transactions(86400).await.unwrap(); // 24小时
        assert!(removed > 0);
        
        let history = processor.get_address_history("0xsender", 10).await;
        assert_eq!(history.len(), 0);
    }

    #[tokio::test]
    async fn test_export_data() {
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
            event_type: "transfer".to_string(),
        };

        processor.process_transfer_event(event).await.unwrap();
        
        let json_data = processor.export_data(ExportFormat::Json).await.unwrap();
        assert!(json_data.contains("balances"));
        assert!(json_data.contains("stats"));
        
        let csv_data = processor.export_data(ExportFormat::Csv).await.unwrap();
        assert!(csv_data.contains("Address,Balance,Total Transactions"));
    }
}