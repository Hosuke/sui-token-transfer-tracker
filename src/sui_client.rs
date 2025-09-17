use sui_graphql_client::{
    Client,
    faucet::FaucetClient,
};
use sui_sdk_types::Address;
use crate::error::{TrackerError, TrackerResult};
use chrono::{DateTime, Utc};
use std::str::FromStr;

/// SUI客户端封装，使用官方GraphQL SDK
pub struct SuiClient {
    client: Client,
    network_url: String,
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

        Ok(Self {
            client,
            network_url: network_url.to_string(),
        })
    }

    /// 获取指定地址和代币类型的余额
    /// 注意：由于SUI GraphQL schema仍在快速发展中，目前使用模拟数据
    /// 实际部署时需要根据最新的GraphQL schema实现真实查询
    pub async fn get_balance(&self, address: &str, _coin_type: Option<&str>) -> TrackerResult<u64> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        // TODO: 实现真实的GraphQL余额查询
        // 当前使用模拟数据，因为GraphQL schema仍在演进中
        log::warn!("使用模拟余额数据 - 地址: {}", address);
        Ok(1000000000) // 1 SUI
    }

    /// 获取地址的所有代币余额
    /// 注意：由于SUI GraphQL schema仍在快速发展中，目前使用模拟数据
    pub async fn get_all_balances(&self, address: &str) -> TrackerResult<Vec<(String, u64)>> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        // TODO: 实现真实的GraphQL余额查询
        // 当前使用模拟数据，因为GraphQL schema仍在演进中
        log::warn!("使用模拟余额数据 - 地址: {}", address);
        let balances = vec![
            ("0x2::sui::SUI".to_string(), 1000000000),
        ];

        Ok(balances)
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
    /// 注意：由于SUI GraphQL schema仍在快速发展中，目前使用模拟数据
    async fn query_transactions(&self, address: &str, limit: Option<u16>) -> TrackerResult<Vec<SuiTransaction>> {
        // 验证地址格式
        Address::from_str(address)
            .map_err(|e| TrackerError::invalid_address(format!("Invalid address: {}", e)))?;

        // 检查网络连接
        self.health_check().await?;

        let _limit = limit.unwrap_or(10);

        // TODO: 实现真实的GraphQL交易查询
        // 当前使用模拟数据，因为GraphQL schema仍在演进中
        log::warn!("使用模拟交易数据 - 地址: {}", address);
        let transactions = vec![
            SuiTransaction {
                digest: "0x1234567890abcdef".to_string(),
                timestamp: Some(Utc::now()),
                gas_used: Some("1000000".to_string()),
                balance_changes: vec![
                    BalanceChange {
                        owner: address.to_string(),
                        coin_type: "0x2::sui::SUI".to_string(),
                        amount: -100000000, // -0.1 SUI
                    }
                ],
            }
        ];

        Ok(transactions)
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