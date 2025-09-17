use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::error::{TrackerError, TrackerResult, utils};
use std::sync::Arc;
use tokio::time::Duration;

#[derive(Debug, Clone)]
pub struct SuiClient {
    client: Arc<Client>,
    network_url: String,
    #[allow(dead_code)]
    timeout: Duration,
}

#[derive(Debug, Serialize, Clone)]
struct SuiRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SuiResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<T>,
    error: Option<SuiError>,
}

#[derive(Debug, Deserialize)]
struct SuiError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct SuiObjectData {
    pub data: SuiObjectInfo,
}

#[derive(Debug, Deserialize)]
pub struct SuiObjectInfo {
    pub content: serde_json::Value,
    pub owner: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BalanceInfo {
    #[serde(rename = "coinType")]
    pub coin_type: String,
    #[serde(rename = "coinObjectCount")]
    pub coin_object_count: u64,
    #[serde(rename = "totalBalance")]
    pub total_balance: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SuiEvent {
    pub id: EventId,
    pub package_id: String,
    pub transaction_module: String,
    pub sender: String,
    pub timestamp: u64,
    pub parsed_json: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventId {
    pub tx_digest: String,
    pub event_seq: u64,
}

#[derive(Debug, Deserialize)]
pub struct Checkpoint {
    pub sequence_number: u64,
    pub timestamp: u64,
}

impl SuiClient {
    pub async fn new(network_url: &str) -> TrackerResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| TrackerError::NetworkError(e))?;

        Ok(Self {
            client: Arc::new(client),
            network_url: network_url.to_string(),
            timeout: Duration::from_secs(30),
        })
    }

    pub async fn with_timeout(network_url: &str, timeout_secs: u64) -> TrackerResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| TrackerError::NetworkError(e))?;

        Ok(Self {
            client: Arc::new(client),
            network_url: network_url.to_string(),
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    async fn rpc_call<T>(&self, method: &str, params: serde_json::Value) -> TrackerResult<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let request = SuiRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let response = utils::retry_operation(
            || {
                let client = self.client.clone();
                let network_url = self.network_url.clone();
                let request = request.clone();
                
                async move {
                    client
                        .post(&network_url)
                        .json(&request)
                        .send()
                        .await
                        .map_err(|e| TrackerError::NetworkError(e))?
                        .json::<SuiResponse<T>>()
                        .await
                        .map_err(|e| TrackerError::NetworkError(e))
                }
            },
            3,
            1000,
        ).await?;

        if let Some(error) = response.error {
            return Err(TrackerError::sui_client_error(
                format!("RPC error {}: {}", error.code, error.message)
            ));
        }

        response.result.ok_or_else(|| {
            TrackerError::sui_client_error("No result in RPC response")
        })
    }

    pub async fn get_balance(&self, address: &str) -> TrackerResult<u64> {
        let params = serde_json::json!([address]);
        let balances: Vec<BalanceInfo> = self.rpc_call("suix_getAllBalances", params).await?;

        for balance in balances {
            if balance.coin_type == "0x2::sui::SUI" {
                return balance.total_balance.parse::<u64>()
                    .map_err(|_| TrackerError::parse_error("Invalid balance format"));
            }
        }

        Ok(0)
    }

    pub async fn query_transfer_events(
        &self,
        address: &str,
        limit: u32,
    ) -> TrackerResult<Vec<SuiEvent>> {
        let params = serde_json::json!({
            "query": {
                "Sender": address
            },
            "limit": limit,
            "order": "descending"
        });

        let events: Vec<SuiEvent> = self.rpc_call("suix_queryEvents", params).await?;
        Ok(events)
    }

    pub async fn get_latest_checkpoint(&self) -> TrackerResult<u64> {
        let params = serde_json::json!(null);
        let checkpoint: Checkpoint = self.rpc_call("sui_getLatestCheckpointSequenceNumber", params).await?;
        Ok(checkpoint.sequence_number)
    }

    pub async fn get_object(&self, object_id: &str) -> TrackerResult<SuiObjectData> {
        let params = serde_json::json!([object_id]);
        let object: SuiObjectData = self.rpc_call("sui_getObject", params).await?;
        Ok(object)
    }

    pub async fn get_transaction(&self, transaction_digest: &str) -> TrackerResult<serde_json::Value> {
        let params = serde_json::json!([transaction_digest]);
        let transaction: serde_json::Value = self.rpc_call("sui_getTransaction", params).await?;
        Ok(transaction)
    }

    pub async fn get_system_state(&self) -> TrackerResult<serde_json::Value> {
        let params = serde_json::json!(null);
        let state: serde_json::Value = self.rpc_call("sui_getLatestSuiSystemState", params).await?;
        Ok(state)
    }

    pub async fn is_healthy(&self) -> bool {
        match self.get_latest_checkpoint().await {
            Ok(_) => true,
            Err(e) => {
                log::warn!("Health check failed: {}", e);
                false
            }
        }
    }

    pub fn network_url(&self) -> &str {
        &self.network_url
    }

    pub async fn estimate_gas_cost(&self, transaction: &serde_json::Value) -> TrackerResult<u64> {
        let params = serde_json::json!([transaction]);
        let gas_cost: serde_json::Value = self.rpc_call("sui_dryRunTransaction", params).await?;
        
        gas_cost["gas_cost"]["computation_cost"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| TrackerError::parse_error("Invalid gas cost format"))
    }
}

pub struct EventStream {
    // 简化版本，实际实现可能需要WebSocket连接
    events: Vec<SuiEvent>,
    index: usize,
}

impl EventStream {
    pub fn new(events: Vec<SuiEvent>) -> Self {
        Self {
            events,
            index: 0,
        }
    }

    pub async fn next(&mut self) -> Option<SuiEvent> {
        if self.index < self.events.len() {
            let event = self.events[self.index].clone();
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SuiClient::new("https://fullnode.mainnet.sui.io:443").await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap();
        assert!(client.is_healthy().await);
    }

    #[tokio::test]
    async fn test_get_latest_checkpoint() {
        let client = SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap();
        let checkpoint = client.get_latest_checkpoint().await;
        assert!(checkpoint.is_ok());
        assert!(checkpoint.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_get_balance() {
        let client = SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap();
        let balance = client.get_balance("0x2").await;
        assert!(balance.is_ok());
    }

    #[tokio::test]
    async fn test_query_transfer_events() {
        let client = SuiClient::new("https://fullnode.mainnet.sui.io:443").await.unwrap();
        let events = client.query_transfer_events("0x2", 5).await;
        assert!(events.is_ok());
    }

    #[tokio::test]
    async fn test_with_timeout() {
        let client = SuiClient::with_timeout("https://fullnode.mainnet.sui.io:443", 5).await.unwrap();
        let checkpoint = client.get_latest_checkpoint().await;
        assert!(checkpoint.is_ok());
    }

    #[test]
    fn test_event_stream() {
        let events = vec![
            SuiEvent {
                id: EventId {
                    tx_digest: "0x123".to_string(),
                    event_seq: 1,
                },
                package_id: "0x456".to_string(),
                transaction_module: "test".to_string(),
                sender: "0x789".to_string(),
                timestamp: 1234567890,
                parsed_json: serde_json::json!({}),
            },
        ];

        let mut stream = EventStream::new(events);
        assert!(stream.next().await.is_some());
        assert!(stream.next().await.is_none());
    }
}