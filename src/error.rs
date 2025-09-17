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
    Configuration(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
    
    #[error("TOML serialize error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl TrackerError {
    pub fn network_error(msg: impl Into<String>) -> Self {
        TrackerError::SuiClientError(msg.into())
    }

    pub fn sui_client_error(msg: impl Into<String>) -> Self {
        TrackerError::SuiClientError(msg.into())
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        TrackerError::ParseError(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        TrackerError::Configuration(msg.into())
    }

    pub fn validation_error(msg: impl Into<String>) -> Self {
        TrackerError::ValidationError(msg.into())
    }

    pub fn database_error(msg: impl Into<String>) -> Self {
        TrackerError::DatabaseError(msg.into())
    }

    pub fn invalid_address(msg: impl Into<String>) -> Self {
        TrackerError::InvalidAddress(msg.into())
    }

    pub fn timeout_error(msg: impl Into<String>) -> Self {
        TrackerError::TimeoutError(msg.into())
    }

    pub fn is_retriable(&self) -> bool {
        match self {
            TrackerError::NetworkError(_) => true,
            TrackerError::TimeoutError(_) => true,
            TrackerError::SuiClientError(_) => true,
            _ => false,
        }
    }

    pub fn error_code(&self) -> u32 {
        match self {
            TrackerError::NetworkError(_) => 1001,
            TrackerError::SuiClientError(_) => 1002,
            TrackerError::ParseError(_) => 2001,
            TrackerError::Configuration(_) => 2002,
            TrackerError::IoError(_) => 3001,
            TrackerError::SerializationError(_) => 3002,
            TrackerError::TomlError(_) => 3003,
            TrackerError::TomlSerializeError(_) => 3004,
            TrackerError::InvalidAddress(_) => 4001,
            TrackerError::TimeoutError(_) => 4002,
            TrackerError::ValidationError(_) => 4003,
            TrackerError::DatabaseError(_) => 5001,
        }
    }
}

pub mod utils {
    use super::*;
    use tokio::time::{sleep, Duration};
    use std::time::Instant;

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
                Err(e) if retries < max_retries && e.is_retriable() => {
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
            Err(_) => Err(TrackerError::timeout_error(
                format!("Operation timed out after {} seconds", timeout_secs)
            )),
        }
    }

    pub fn measure_time<F, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        (result, duration)
    }

    pub async fn measure_async_time<F, Fut, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        (result, duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(TrackerError::network_error("test").error_code(), 1001);
        assert_eq!(TrackerError::config_error("test").error_code(), 2002);
        assert_eq!(TrackerError::invalid_address("test").error_code(), 4001);
    }

    #[test]
    fn test_retriable_errors() {
        assert!(TrackerError::network_error("test").is_retriable());
        assert!(TrackerError::timeout_error("test").is_retriable());
        assert!(!TrackerError::config_error("test").is_retriable());
        assert!(!TrackerError::invalid_address("test").is_retriable());
    }

    #[tokio::test]
    async fn test_retry_operation_success() {
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
    async fn test_retry_operation_failure() {
        let result = utils::retry_operation(
            || async {
                Err(TrackerError::config_error("Permanent failure"))
            },
            3,
            10,
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TrackerError::Configuration(_)));
    }

    #[tokio::test]
    async fn test_timeout() {
        let result = utils::with_timeout(
            async {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                Ok::<(), TrackerError>(())
            },
            1,
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TrackerError::TimeoutError(_)));
    }

    #[test]
    fn test_measure_time() {
        let (result, duration) = utils::measure_time(|| {
            std::thread::sleep(StdDuration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        assert!(duration >= StdDuration::from_millis(10));
    }
}