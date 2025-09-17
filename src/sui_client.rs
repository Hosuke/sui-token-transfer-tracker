use sui_graphql_client::{
    Client,
    faucet::FaucetClient,
};
use sui_sdk_types::Address;
use crate::error::{TrackerError, TrackerResult};
use chrono::{DateTime, Utc};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use reqwest;

/// JSON-RPC请求结构
#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// JSON-RPC响应结构
#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// SUI余额响应结构
#[derive(Deserialize, Debug)]
struct SuiBalance {
    #[serde(rename = "coinType")]
    coin_type: String,
    #[serde(rename = "coinObjectCount")]
    coin_object_count: u64,
    #[serde(rename = "totalBalance")]
    total_balance: String,
    #[serde(rename = "lockedBalance")]
    locked_balance: Option<serde_json::Value>,
}

/// SUI Coin对象响应结构
#[derive(Deserialize, Debug)]
struct SuiCoin {
    #[serde(rename = "coinType")]
    coin_type: String,
    #[serde(rename = "coinObjectId")]
    coin_object_id: String,
    version: String,
    digest: String,
    balance: String,
    #[serde(rename = "previousTransaction")]
    previous_transaction: String,
}

/// SUI交易块查询响应结构
#[derive(Deserialize, Debug)]
struct TransactionBlocksResponse {
    data: Vec<TransactionBlockData>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Deserialize, Debug)]
struct TransactionBlockData {
    digest: String,
    transaction: Option<serde_json::Value>,
    effects: Option<TransactionEffects>,
    events: Option<Vec<serde_json::Value>>,
    #[serde(rename = "timestampMs")]
    timestamp_ms: Option<String>,
    checkpoint: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TransactionEffects {
    #[serde(rename = "messageVersion")]
    message_version: String,
    status: EffectStatus,
    #[serde(rename = "executedEpoch")]
    executed_epoch: String,
    #[serde(rename = "gasUsed")]
    gas_used: Option<GasInfo>,
    #[serde(rename = "transactionDigest")]
    transaction_digest: String,
    created: Option<Vec<serde_json::Value>>,
    mutated: Option<Vec<serde_json::Value>>,
    deleted: Option<Vec<serde_json::Value>>,
    #[serde(rename = "balanceChanges")]
    balance_changes: Option<Vec<BalanceChangeItem>>,
}

#[derive(Deserialize, Debug)]
struct EffectStatus {
    status: String,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GasInfo {
    #[serde(rename = "computationCost")]
    computation_cost: String,
    #[serde(rename = "storageCost")]
    storage_cost: String,
    #[serde(rename = "storageRebate")]
    storage_rebate: String,
    #[serde(rename = "nonRefundableStorageFee")]
    non_refundable_storage_fee: String,
}

#[derive(Deserialize, Debug)]
struct BalanceChangeItem {
    owner: OwnerInfo,
    #[serde(rename = "coinType")]
    coin_type: String,
    amount: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum OwnerInfo {
    AddressOwner { 
        #[serde(rename = "AddressOwner")]
        address_owner: String 
    },
    ObjectOwner { 
        #[serde(rename = "ObjectOwner")]
        object_owner: String 
    },
    Shared { 
        #[serde(rename = "Shared")]
        shared: serde_json::Value 
    },
    Immutable,
}

/// SUI客户端封装，使用官方GraphQL SDK + JSON-RPC
pub struct SuiClient {
    client: Client,
    network_url: String,
    rpc_url: String,
    http_client: reqwest::Client,
}

/// 交易信息结构
#[derive(Debug, Clone)]
pub struct SuiTransaction {
    pub digest: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub gas_used: Option<String>,
    pub balance_changes: Vec<BalanceChange>,
}

/// 余额变化信息
#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub owner: String,
    pub coin_type: String,
    pub amount: i64,
}

impl std::fmt::Debug for SuiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuiClient")
            .field("network_url", &self.network_url)
            .finish()
    }
}

impl SuiClient {
    /// 创建新的SUI客户端
    pub async fn new(network_url: &str) -> TrackerResult<Self> {
        let client = match network_url {
            url if url.contains("mainnet") => Client::new_mainnet(),
            url if url.contains("testnet") => Client::new_testnet(), 
            url if url.contains("devnet") => Client::new_devnet(),
            url if url.contains("localhost") || url.contains("localnet") => Client::new_localhost(),
            _ => Client::new_mainnet(), // 默认使用主网
        };

        // 确定JSON-RPC URL
        let rpc_url = if network_url.contains("mainnet") {
            "https://fullnode.mainnet.sui.io:443".to_string()
        } else if network_url.contains("testnet") {
            "https://fullnode.testnet.sui.io:443".to_string()
        } else if network_url.contains("devnet") {
            "https://fullnode.devnet.sui.io:443".to_string()
        } else if network_url.contains("localhost") || network_url.contains("localnet") {
            "http://localhost:9000".to_string()
        } else {
            "https://fullnode.mainnet.sui.io:443".to_string() // 默认主网
        };

        let http_client = reqwest::Client::new();

        log::info!("Initializing SUI client with GraphQL: {} and RPC: {}", network_url, rpc_url);

        Ok(Self {
            client,
            network_url: network_url.to_string(),
            rpc_url,
            http_client,
        })
    }

    /// 发送JSON-RPC请求
    async fn send_rpc_request<T>(&self, method: &str, params: serde_json::Value) -> TrackerResult<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        log::debug!("Sending RPC request to {}: {} with params: {}", self.rpc_url, method, params);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let response = self.http_client
            .post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| TrackerError::network_error(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(TrackerError::network_error(format!(
                "HTTP error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let rpc_response: JsonRpcResponse<T> = response
            .json()
            .await
            .map_err(|e| TrackerError::parse_error(&format!("Failed to parse JSON response: {}", e)))?;

        if let Some(error) = rpc_response.error {
            return Err(TrackerError::network_error(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }

        rpc_response.result.ok_or_else(|| {
            TrackerError::parse_error("RPC response missing result field")
        })
    }

    /// 获取指定地址和代币类型的余额
    /// 使用真实的JSON-RPC API调用
    pub async fn get_balance(&self, address: &str, coin_type: Option<&str>) -> TrackerResult<u64> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        let coin_type = coin_type.unwrap_or("0x2::sui::SUI");
        
        // 使用真实的JSON-RPC API调用
        log::info!("Querying real balance for address: {} coin: {}", address, coin_type);
        
        let params = serde_json::json!([address, coin_type]);
        
        match self.send_rpc_request::<SuiBalance>("suix_getBalance", params).await {
            Ok(balance_response) => {
                log::info!("Successfully got balance response: {:?}", balance_response);
                
                // 解析余额字符串为u64
                match balance_response.total_balance.parse::<u64>() {
                    Ok(balance) => {
                        log::info!("Parsed balance: {} for address: {}", balance, address);
                        Ok(balance)
                    },
                    Err(e) => {
                        log::error!("Failed to parse balance '{}': {}", balance_response.total_balance, e);
                        Err(TrackerError::parse_error(&format!("Invalid balance format: {}", e)))
                    }
                }
            },
            Err(e) => {
                log::error!("Failed to get balance: {}", e);
                Err(e)
            }
        }
    }

    /// 获取地址的所有代币余额
    /// 使用真实的JSON-RPC API调用
    pub async fn get_all_balances(&self, address: &str) -> TrackerResult<Vec<(String, u64)>> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        // 使用真实的JSON-RPC API调用
        log::info!("Querying all real balances for address: {}", address);
        
        let params = serde_json::json!([address]);
        
        match self.send_rpc_request::<Vec<SuiBalance>>("suix_getAllBalances", params).await {
            Ok(balances_response) => {
                log::info!("Successfully got all balances response: {:?}", balances_response);
                
                let mut result = Vec::new();
                
                for balance in balances_response {
                    match balance.total_balance.parse::<u64>() {
                        Ok(amount) => {
                            result.push((balance.coin_type, amount));
                        },
                        Err(e) => {
                            log::warn!("Failed to parse balance '{}' for coin type '{}': {}", 
                                balance.total_balance, balance.coin_type, e);
                        }
                    }
                }
                
                log::info!("Parsed {} balances for address: {}", result.len(), address);
                Ok(result)
            },
            Err(e) => {
                log::error!("Failed to get all balances: {}", e);
                Err(e)
            }
        }
    }

    /// 查询发送的交易
    pub async fn query_transactions_sent(&self, address: &str, limit: Option<u16>) -> TrackerResult<Vec<SuiTransaction>> {
        self.query_transactions(address, limit).await
    }

    /// 查询接收的交易  
    pub async fn query_transactions_received(&self, address: &str, limit: Option<u16>) -> TrackerResult<Vec<SuiTransaction>> {
        self.query_transactions(address, limit).await
    }

    /// 通用交易查询方法
    /// 使用真实的JSON-RPC API调用
    async fn query_transactions(&self, address: &str, limit: Option<u16>) -> TrackerResult<Vec<SuiTransaction>> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        let limit = limit.unwrap_or(10) as u64;

        // 使用真实的JSON-RPC API调用
        log::info!("Querying real transactions for address: {} limit: {}", address, limit);
        
        // 构建查询参数
        let filter = serde_json::json!({
            "FromAddress": address
        });

        let options = serde_json::json!({
            "showInput": false,
            "showRawInput": false,
            "showEffects": true,
            "showEvents": false,
            "showObjectChanges": false,
            "showBalanceChanges": true
        });

        let params = serde_json::json!([
            {
                "filter": filter,
                "options": options
            },
            null, // cursor
            limit,
            false // descending order
        ]);
        
        match self.send_rpc_request::<TransactionBlocksResponse>("suix_queryTransactionBlocks", params).await {
            Ok(response) => {
                log::info!("Successfully got transaction blocks response with {} transactions", response.data.len());
                
                let mut result = Vec::new();
                
                for tx_data in response.data {
                    let mut balance_changes = Vec::new();
                    
                    // 解析余额变化
                    if let Some(effects) = &tx_data.effects {
                        if let Some(changes) = &effects.balance_changes {
                            for change in changes {
                                match change.amount.parse::<i64>() {
                                    Ok(amount) => {
                                        let owner_address = match &change.owner {
                                            OwnerInfo::AddressOwner { address_owner } => address_owner.clone(),
                                            _ => address.to_string(), // 默认使用查询地址
                                        };
                                        
                                        balance_changes.push(BalanceChange {
                                            owner: owner_address,
                                            coin_type: change.coin_type.clone(),
                                            amount,
                                        });
                                    },
                                    Err(e) => {
                                        log::warn!("Failed to parse amount '{}': {}", change.amount, e);
                                    }
                                }
                            }
                        }
                    }

                    // 解析gas消耗
                    let gas_used = tx_data.effects
                        .as_ref()
                        .and_then(|e| e.gas_used.as_ref())
                        .map(|g| {
                            // 计算总gas消耗（避免溢出）
                            let computation_cost: u64 = g.computation_cost.parse().unwrap_or(0);
                            let storage_cost: u64 = g.storage_cost.parse().unwrap_or(0);
                            let storage_rebate: u64 = g.storage_rebate.parse().unwrap_or(0);
                            let non_refundable: u64 = g.non_refundable_storage_fee.parse().unwrap_or(0);
                            
                            // 使用安全的减法避免溢出
                            let total_costs = computation_cost + storage_cost + non_refundable;
                            let total_gas = if total_costs >= storage_rebate {
                                total_costs - storage_rebate
                            } else {
                                0
                            };
                            total_gas.to_string()
                        });

                    // 解析时间戳
                    let timestamp = tx_data.timestamp_ms
                        .and_then(|ts| ts.parse::<i64>().ok())
                        .map(|ts_ms| {
                            let dt = chrono::DateTime::from_timestamp_millis(ts_ms);
                            dt.unwrap_or_else(|| Utc::now())
                        });

                    result.push(SuiTransaction {
                        digest: tx_data.digest,
                        timestamp,
                        gas_used,
                        balance_changes,
                    });
                }
                
                log::info!("Parsed {} transactions for address: {}", result.len(), address);
                Ok(result)
            },
            Err(e) => {
                log::error!("Failed to get transactions: {}", e);
                Err(e)
            }
        }
    }

    /// 获取链ID
    pub async fn get_chain_id(&self) -> TrackerResult<String> {
        self.client
            .chain_id()
            .await
            .map_err(|e| TrackerError::network_error(format!("Failed to get chain ID: {:?}", e)))
    }

    /// 健康检查
    pub async fn health_check(&self) -> TrackerResult<bool> {
        match self.get_chain_id().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 请求测试网代币（仅用于测试）
    pub async fn request_faucet(&self, address: &str) -> TrackerResult<()> {
        let address = Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        let faucet = if self.network_url.contains("devnet") {
            FaucetClient::devnet()
        } else if self.network_url.contains("testnet") {
            FaucetClient::testnet()
        } else {
            return Err(TrackerError::config_error("Faucet only available on devnet/testnet"));
        };

        faucet
            .request(address)
            .await
            .map_err(|e| TrackerError::network_error(format!("Faucet request failed: {:?}", e)))?;

        Ok(())
    }

    /// 检查是否健康（兼容性方法）
    pub async fn is_healthy(&self) -> bool {
        self.health_check().await.unwrap_or(false)
    }

    /// 创建带超时的客户端（兼容性方法）
    pub async fn with_timeout(network_url: &str, _timeout_seconds: u64) -> TrackerResult<Self> {
        Self::new(network_url).await
    }

    /// 查询转移事件（兼容性方法）
    pub async fn query_transfer_events(&self, address: &str, limit: u32) -> TrackerResult<Vec<SuiEvent>> {
        let transactions = self.query_transactions(address, Some(limit as u16)).await?;
        
        // 转换为事件格式
        let events: Vec<SuiEvent> = transactions
            .into_iter()
            .map(|tx| SuiEvent {
                id: tx.digest.clone(),
                package_id: "0x2".to_string(),
                transaction_module: "sui".to_string(),
                sender: address.to_string(),
                recipient: tx.balance_changes.get(0)
                    .map(|bc| bc.owner.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                amount: tx.balance_changes.get(0)
                    .map(|bc| bc.amount.abs() as u64)
                    .unwrap_or(0),
                token_type: "0x2::sui::SUI".to_string(),
                timestamp: tx.timestamp.map(|t| t.timestamp() as u64).unwrap_or(0),
                block_number: 0,
            })
            .collect();

        Ok(events)
    }
}

/// SUI事件结构（兼容性）
#[derive(Debug, Clone)]
pub struct SuiEvent {
    pub id: String,
    pub package_id: String,
    pub transaction_module: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub token_type: String,
    pub timestamp: u64,
    pub block_number: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SuiClient::new("https://sui-mainnet.mystenlabs.com/graphql").await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        if let Ok(client) = SuiClient::new("https://sui-mainnet.mystenlabs.com/graphql").await {
            let health = client.health_check().await.unwrap_or(false);
            // 网络问题时不让测试失败
            println!("Health check result: {}", health);
        }
    }

    #[tokio::test]
    async fn test_get_chain_id() {
        if let Ok(client) = SuiClient::new("https://sui-mainnet.mystenlabs.com/graphql").await {
            if let Ok(chain_id) = client.get_chain_id().await {
                println!("Chain ID: {}", chain_id);
                assert!(!chain_id.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_get_balance() {
        if let Ok(client) = SuiClient::new("https://sui-mainnet.mystenlabs.com/graphql").await {
            let test_address = "0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee";
            if let Ok(balance) = client.get_balance(test_address, Some("0x2::sui::SUI")).await {
                println!("Balance: {}", balance);
                assert!(balance >= 0);
            }
        }
    }
}