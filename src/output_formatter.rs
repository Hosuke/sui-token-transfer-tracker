use crate::transaction_processor::{Transaction, AddressStats, ProcessorStats};
use crate::alert_system::{Alert, AlertStats};
use std::collections::HashMap;
use chrono::DateTime;

#[derive(Debug, Clone)]
pub struct OutputFormatter {
    config: OutputConfig,
    use_colors: bool,
    show_timestamps: bool,
    output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub use_colors: bool,
    pub show_timestamps: bool,
    pub max_recent_transactions: u32,
    pub balance_summary_interval: u64,
    pub table_width: usize,
    pub enable_json_output: bool,
    pub enable_csv_output: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_timestamps: true,
            max_recent_transactions: 10,
            balance_summary_interval: 300,
            table_width: 80,
            enable_json_output: false,
            enable_csv_output: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

impl OutputFormatter {
    pub fn new(use_colors: bool, show_timestamps: bool) -> Self {
        Self {
            config: OutputConfig::default(),
            use_colors,
            show_timestamps,
            output_format: OutputFormat::Table,
        }
    }

    pub fn with_config(config: OutputConfig) -> Self {
        Self {
            config: config.clone(),
            use_colors: config.use_colors,
            show_timestamps: config.show_timestamps,
            output_format: OutputFormat::Table,
        }
    }

    pub fn set_format(&mut self, format: OutputFormat) {
        self.output_format = format;
    }

    pub fn format_transaction(&self, transaction: &Transaction) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_transaction_table(transaction),
            OutputFormat::Json => self.format_transaction_json(transaction),
            OutputFormat::Csv => self.format_transaction_csv(transaction),
        }
    }

    pub fn format_alert(&self, alert: &Alert) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_alert_table(alert),
            OutputFormat::Json => self.format_alert_json(alert),
            OutputFormat::Csv => self.format_alert_csv(alert),
        }
    }

    pub fn format_balance_summary(&self, balances: &HashMap<String, u64>) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_balance_summary_table(balances),
            OutputFormat::Json => self.format_balance_summary_json(balances),
            OutputFormat::Csv => self.format_balance_summary_csv(balances),
        }
    }

    pub fn format_transaction_history(&self, transactions: &[Transaction]) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_transaction_history_table(transactions),
            OutputFormat::Json => self.format_transaction_history_json(transactions),
            OutputFormat::Csv => self.format_transaction_history_csv(transactions),
        }
    }

    pub fn format_address_stats(&self, address: &str, stats: &AddressStats) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_address_stats_table(address, stats),
            OutputFormat::Json => self.format_address_stats_json(address, stats),
            OutputFormat::Csv => self.format_address_stats_csv(address, stats),
        }
    }

    pub fn format_system_stats(&self, stats: &ProcessorStats) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_system_stats_table(stats),
            OutputFormat::Json => self.format_system_stats_json(stats),
            OutputFormat::Csv => self.format_system_stats_csv(stats),
        }
    }

    pub fn format_alert_summary(&self, alert_stats: &AlertStats) -> String {
        match self.output_format {
            OutputFormat::Table => self.format_alert_summary_table(alert_stats),
            OutputFormat::Json => self.format_alert_summary_json(alert_stats),
            OutputFormat::Csv => self.format_alert_summary_csv(alert_stats),
        }
    }

    // Table formatting methods
    fn format_transaction_table(&self, transaction: &Transaction) -> String {
        let timestamp = if self.show_timestamps {
            let dt = DateTime::from_timestamp(transaction.timestamp as i64, 0)
                .unwrap_or_default();
            format!("{} ", dt.format("%H:%M:%S"))
        } else {
            String::new()
        };

        let amount_formatted = self.format_amount(transaction.amount);
        let _color_prefix = if self.use_colors {
            self.get_transaction_color(transaction)
        } else {
            String::new()
        };

        let _color_suffix = if self.use_colors {
            "\x1b[0m"
        } else {
            ""
        };

        format!(
            "{}{}→ {} {} | {} | {} | {}",
            timestamp,
            self.truncate_address(&transaction.sender),
            self.truncate_address(&transaction.recipient),
            amount_formatted,
            transaction.token_type,
            self.format_status(&transaction.status),
            self.truncate_id(&transaction.id)
        )
    }

    fn format_alert_table(&self, alert: &Alert) -> String {
        let timestamp = if self.show_timestamps {
            format!("{} ", alert.timestamp().format("%H:%M:%S"))
        } else {
            String::new()
        };

        let severity = match alert.severity() {
            crate::alert_system::AlertSeverity::Info => "INFO",
            crate::alert_system::AlertSeverity::Warning => "WARN",
            crate::alert_system::AlertSeverity::Error => "ERROR",
            crate::alert_system::AlertSeverity::Critical => "CRIT",
        };

        let severity_color = if self.use_colors {
            match alert.severity() {
                crate::alert_system::AlertSeverity::Info => "\x1b[34m",    // Blue
                crate::alert_system::AlertSeverity::Warning => "\x1b[33m", // Yellow
                crate::alert_system::AlertSeverity::Error => "\x1b[31m",  // Red
                crate::alert_system::AlertSeverity::Critical => "\x1b[91m", // Bright Red
            }
        } else {
            ""
        };

        let color_suffix = if self.use_colors {
            "\x1b[0m"
        } else {
            ""
        };

        let message = match alert {
            Alert::LowBalance { address, balance, threshold, .. } => {
                format!("Low balance: {} | Balance: {} | Threshold: {}",
                    self.truncate_address(address),
                    self.format_amount(*balance),
                    self.format_amount(*threshold))
            },
            Alert::LargeTransfer { sender, recipient, amount, token_type, .. } => {
                format!("Large transfer: {} → {} | {} {}",
                    self.truncate_address(sender),
                    self.truncate_address(recipient),
                    self.format_amount(*amount),
                    token_type)
            },
            Alert::SuspiciousActivity { address, activity_type, description, .. } => {
                format!("Suspicious activity: {} | {} | {}",
                    self.truncate_address(address),
                    activity_type,
                    description)
            },
            Alert::NetworkError { error, component, .. } => {
                format!("Network error in {}: {}", component, error)
            },
            Alert::SystemError { error, component, .. } => {
                format!("System error in {}: {}", component, error)
            },
            Alert::Custom { title, message, .. } => {
                format!("{}: {}", title, message)
            },
        };

        format!(
            "{}[{}] {}{}{}",
            timestamp,
            severity_color,
            severity,
            color_suffix,
            message
        )
    }

    fn format_balance_summary_table(&self, balances: &HashMap<String, u64>) -> String {
        if balances.is_empty() {
            return "No balances to display".to_string();
        }

        let mut summary = String::from("Balance Summary:\n");
        summary.push_str(&format!("{:<20} {:<15} {:<10}\n", "Address", "Balance (SUI)", "Balance"));
        summary.push_str(&format!("{:<20} {:<15} {:<10}\n", 
            self.repeat_char('=', 20), 
            self.repeat_char('=', 15), 
            self.repeat_char('=', 10)));

        let mut sorted_balances: Vec<_> = balances.iter().collect();
        sorted_balances.sort_by(|a, b| b.1.cmp(a.1));

        for (address, balance) in sorted_balances {
            summary.push_str(&format!(
                "{:<20} {:<15.9} {:<10}\n",
                self.truncate_address(address),
                *balance as f64 / 1_000_000_000.0,
                self.format_amount(*balance)
            ));
        }

        summary
    }

    fn format_transaction_history_table(&self, transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return "No transactions to display".to_string();
        }

        let mut history = String::from("Recent Transactions:\n");
        history.push_str(&format!("{:<12} {:<12} {:<12} {:<15} {:<12} {:<8}\n", 
            "Time", "From", "To", "Amount (SUI)", "Token", "Status"));
        history.push_str(&format!("{:<12} {:<12} {:<12} {:<15} {:<12} {:<8}\n", 
            self.repeat_char('=', 12), 
            self.repeat_char('=', 12), 
            self.repeat_char('=', 12), 
            self.repeat_char('=', 15), 
            self.repeat_char('=', 12), 
            self.repeat_char('=', 8)));

        for transaction in transactions.iter().take(self.config.max_recent_transactions as usize) {
            let dt = DateTime::from_timestamp(transaction.timestamp as i64, 0)
                .unwrap_or_default();
            
            history.push_str(&format!(
                "{:<12} {:<12} {:<12} {:<15.9} {:<12} {:<8}\n",
                dt.format("%H:%M:%S"),
                self.truncate_address(&transaction.sender),
                self.truncate_address(&transaction.recipient),
                transaction.amount as f64 / 1_000_000_000.0,
                self.format_token_type(&transaction.token_type),
                self.format_status(&transaction.status)
            ));
        }

        history
    }

    fn format_address_stats_table(&self, address: &str, stats: &AddressStats) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("Statistics for {}:\n", self.truncate_address(address)));
        summary.push_str(&format!("  Total Transactions: {}\n", stats.total_transactions));
        summary.push_str(&format!("  Total Sent: {}\n", self.format_amount(stats.total_sent)));
        summary.push_str(&format!("  Total Received: {}\n", self.format_amount(stats.total_received)));
        summary.push_str(&format!("  Average Transaction: {}\n", self.format_amount(stats.average_transaction_amount)));
        summary.push_str(&format!("  Largest Transaction: {}\n", self.format_amount(stats.largest_transaction)));
        summary.push_str(&format!("  Smallest Transaction: {}\n", 
            if stats.smallest_transaction == u64::MAX {
                "N/A".to_string()
            } else {
                self.format_amount(stats.smallest_transaction)
            }));
        
        if let Some(first) = stats.first_transaction {
            let dt = DateTime::from_timestamp(first as i64, 0).unwrap_or_default();
            summary.push_str(&format!("  First Transaction: {}\n", dt.format("%Y-%m-%d %H:%M:%S")));
        }
        
        if let Some(last) = stats.last_transaction {
            let dt = DateTime::from_timestamp(last as i64, 0).unwrap_or_default();
            summary.push_str(&format!("  Last Transaction: {}\n", dt.format("%Y-%m-%d %H:%M:%S")));
        }

        summary
    }

    fn format_system_stats_table(&self, stats: &ProcessorStats) -> String {
        let mut summary = String::from("System Statistics:\n");
        summary.push_str(&format!("  Total Addresses: {}\n", stats.total_addresses));
        summary.push_str(&format!("  Total Transactions: {}\n", stats.total_transactions));
        summary.push_str(&format!("  Total Volume: {}\n", self.format_amount(stats.total_volume)));
        summary.push_str(&format!("  Max History Records: {}\n", stats.config.max_history_records));
        summary.push_str(&format!("  Cleanup Interval: {} hours\n", stats.config.cleanup_interval_hours));
        summary
    }

    fn format_alert_summary_table(&self, alert_stats: &AlertStats) -> String {
        let mut summary = String::from("Alert Summary:\n");
        summary.push_str(&format!("  Total Alerts: {}\n", alert_stats.total_alerts));
        
        summary.push_str("  Alerts by Type:\n");
        for (alert_type, count) in &alert_stats.alerts_by_type {
            summary.push_str(&format!("    {}: {}\n", alert_type, count));
        }
        
        summary.push_str("  Alerts by Severity:\n");
        for (severity, count) in &alert_stats.alerts_by_severity {
            summary.push_str(&format!("    {}: {}\n", severity, count));
        }

        summary
    }

    // JSON formatting methods
    fn format_transaction_json(&self, transaction: &Transaction) -> String {
        serde_json::json!({
            "id": transaction.id,
            "sender": transaction.sender,
            "recipient": transaction.recipient,
            "amount": transaction.amount,
            "amount_sui": transaction.amount as f64 / 1_000_000_000.0,
            "token_type": transaction.token_type,
            "timestamp": transaction.timestamp,
            "block_number": transaction.block_number,
            "gas_used": transaction.gas_used,
            "gas_price": transaction.gas_price,
            "status": self.format_status(&transaction.status),
        }).to_string()
    }

    fn format_alert_json(&self, alert: &Alert) -> String {
        serde_json::json!({
            "type": match alert {
                Alert::LowBalance { .. } => "low_balance",
                Alert::LargeTransfer { .. } => "large_transfer",
                Alert::SuspiciousActivity { .. } => "suspicious_activity",
                Alert::NetworkError { .. } => "network_error",
                Alert::SystemError { .. } => "system_error",
                Alert::Custom { .. } => "custom",
            },
            "severity": match alert.severity() {
                crate::alert_system::AlertSeverity::Info => "info",
                crate::alert_system::AlertSeverity::Warning => "warning",
                crate::alert_system::AlertSeverity::Error => "error",
                crate::alert_system::AlertSeverity::Critical => "critical",
            },
            "timestamp": alert.timestamp().to_rfc3339(),
            "message": self.format_alert_table(alert),
        }).to_string()
    }

    fn format_balance_summary_json(&self, balances: &HashMap<String, u64>) -> String {
        let formatted_balances: HashMap<String, serde_json::Value> = balances
            .iter()
            .map(|(addr, balance)| {
                (addr.clone(), serde_json::json!({
                    "balance": balance,
                    "balance_sui": *balance as f64 / 1_000_000_000.0,
                }))
            })
            .collect();

        serde_json::json!({
            "summary": formatted_balances,
            "total_addresses": balances.len(),
        }).to_string()
    }

    fn format_transaction_history_json(&self, transactions: &[Transaction]) -> String {
        let formatted_transactions: Vec<serde_json::Value> = transactions
            .iter()
            .map(|tx| serde_json::json!({
                "id": tx.id,
                "sender": tx.sender,
                "recipient": tx.recipient,
                "amount": tx.amount,
                "amount_sui": tx.amount as f64 / 1_000_000_000.0,
                "token_type": tx.token_type,
                "timestamp": tx.timestamp,
                "block_number": tx.block_number,
                "gas_used": tx.gas_used,
                "gas_price": tx.gas_price,
                "status": self.format_status(&tx.status),
            }))
            .collect();

        serde_json::json!({
            "transactions": formatted_transactions,
            "total_count": transactions.len(),
        }).to_string()
    }

    fn format_address_stats_json(&self, address: &str, stats: &AddressStats) -> String {
        serde_json::json!({
            "address": address,
            "total_transactions": stats.total_transactions,
            "total_sent": stats.total_sent,
            "total_received": stats.total_received,
            "total_sent_sui": stats.total_sent as f64 / 1_000_000_000.0,
            "total_received_sui": stats.total_received as f64 / 1_000_000_000.0,
            "average_transaction_amount": stats.average_transaction_amount,
            "average_transaction_amount_sui": stats.average_transaction_amount as f64 / 1_000_000_000.0,
            "largest_transaction": stats.largest_transaction,
            "largest_transaction_sui": stats.largest_transaction as f64 / 1_000_000_000.0,
            "smallest_transaction": if stats.smallest_transaction == u64::MAX {
                serde_json::Value::Null
            } else {
                serde_json::json!(stats.smallest_transaction)
            },
            "first_transaction": stats.first_transaction,
            "last_transaction": stats.last_transaction,
        }).to_string()
    }

    fn format_system_stats_json(&self, stats: &ProcessorStats) -> String {
        serde_json::json!({
            "total_addresses": stats.total_addresses,
            "total_transactions": stats.total_transactions,
            "total_volume": stats.total_volume,
            "total_volume_sui": stats.total_volume as f64 / 1_000_000_000.0,
            "max_history_records": stats.config.max_history_records,
            "cleanup_interval_hours": stats.config.cleanup_interval_hours,
        }).to_string()
    }

    fn format_alert_summary_json(&self, alert_stats: &AlertStats) -> String {
        serde_json::json!({
            "total_alerts": alert_stats.total_alerts,
            "alerts_by_type": alert_stats.alerts_by_type,
            "alerts_by_severity": alert_stats.alerts_by_severity,
        }).to_string()
    }

    // CSV formatting methods
    fn format_transaction_csv(&self, transaction: &Transaction) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            transaction.id,
            transaction.sender,
            transaction.recipient,
            transaction.amount,
            transaction.amount as f64 / 1_000_000_000.0,
            transaction.token_type,
            transaction.timestamp,
            transaction.block_number,
            transaction.gas_used.unwrap_or(0),
            self.format_status(&transaction.status)
        )
    }

    fn format_alert_csv(&self, alert: &Alert) -> String {
        format!(
            "{},{},{},{}\n",
            alert.timestamp().to_rfc3339(),
            match alert.severity() {
                crate::alert_system::AlertSeverity::Info => "info",
                crate::alert_system::AlertSeverity::Warning => "warning",
                crate::alert_system::AlertSeverity::Error => "error",
                crate::alert_system::AlertSeverity::Critical => "critical",
            },
            match alert {
                Alert::LowBalance { .. } => "low_balance",
                Alert::LargeTransfer { .. } => "large_transfer",
                Alert::SuspiciousActivity { .. } => "suspicious_activity",
                Alert::NetworkError { .. } => "network_error",
                Alert::SystemError { .. } => "system_error",
                Alert::Custom { .. } => "custom",
            },
            self.format_alert_table(alert)
        )
    }

    fn format_balance_summary_csv(&self, balances: &HashMap<String, u64>) -> String {
        let mut csv = String::from("Address,Balance,Balance_SUI\n");
        for (address, balance) in balances {
            csv.push_str(&format!(
                "{},{},{:.9}\n",
                address,
                balance,
                *balance as f64 / 1_000_000_000.0
            ));
        }
        csv
    }

    fn format_transaction_history_csv(&self, transactions: &[Transaction]) -> String {
        let mut csv = String::from("ID,Sender,Recipient,Amount,Amount_SUI,Token_Type,Timestamp,Block_Number,Gas_Used,Gas_Price,Status\n");
        for tx in transactions {
            csv.push_str(&format!(
                "{},{},{},{},{:.9},{},{},{},{},{},{}\n",
                tx.id,
                tx.sender,
                tx.recipient,
                tx.amount,
                tx.amount as f64 / 1_000_000_000.0,
                tx.token_type,
                tx.timestamp,
                tx.block_number,
                tx.gas_used.unwrap_or(0),
                tx.gas_price.unwrap_or(0),
                self.format_status(&tx.status)
            ));
        }
        csv
    }

    fn format_address_stats_csv(&self, address: &str, stats: &AddressStats) -> String {
        format!(
            "Address,Total_Transactions,Total_Sent,Total_Received,Avg_Transaction,Largest_Transaction,Smallest_Transaction,First_Transaction,Last_Transaction\n{},{},{},{},{:.9},{:.9},{},{},{}\n",
            address,
            stats.total_transactions,
            stats.total_sent,
            stats.total_received,
            stats.average_transaction_amount as f64 / 1_000_000_000.0,
            stats.largest_transaction as f64 / 1_000_000_000.0,
            if stats.smallest_transaction == u64::MAX {
                "N/A".to_string()
            } else {
                stats.smallest_transaction.to_string()
            },
            stats.first_transaction.unwrap_or(0),
            stats.last_transaction.unwrap_or(0)
        )
    }

    fn format_system_stats_csv(&self, stats: &ProcessorStats) -> String {
        format!(
            "Total Addresses,Total Transactions,Total Volume,Total Volume SUI,Max History Records,Cleanup Interval Hours\n{},{}.{:09},{:.9},{},{}\n",
            stats.total_addresses,
            stats.total_transactions,
            stats.total_volume,
            stats.total_volume as f64 / 1_000_000_000.0,
            stats.config.max_history_records,
            stats.config.cleanup_interval_hours
        )
    }

    fn format_alert_summary_csv(&self, alert_stats: &AlertStats) -> String {
        let mut csv = String::from("Total Alerts\n");
        csv.push_str(&format!("{}\n", alert_stats.total_alerts));
        
        csv.push_str("Alerts by Type\n");
        for (alert_type, count) in &alert_stats.alerts_by_type {
            csv.push_str(&format!("{},{}\n", alert_type, count));
        }
        
        csv.push_str("Alerts by Severity\n");
        for (severity, count) in &alert_stats.alerts_by_severity {
            csv.push_str(&format!("{},{}\n", severity, count));
        }
        
        csv
    }

    // Helper methods
    pub fn format_amount(&self, amount: u64) -> String {
        format!("{:.9} SUI", amount as f64 / 1_000_000_000.0)
    }

    fn format_token_type(&self, token_type: &str) -> String {
        if token_type == "0x2::sui::SUI" {
            "SUI".to_string()
        } else {
            token_type.split("::").last().unwrap_or(token_type).to_string()
        }
    }

    fn format_status(&self, status: &crate::transaction_processor::TransactionStatus) -> String {
        match status {
            crate::transaction_processor::TransactionStatus::Success => "✓",
            crate::transaction_processor::TransactionStatus::Failed => "✗",
            crate::transaction_processor::TransactionStatus::Pending => "⏳",
        }.to_string()
    }

    fn truncate_address(&self, address: &str) -> String {
        if address.len() > 10 {
            format!("{}...{}", &address[..6], &address[address.len()-4..])
        } else {
            address.to_string()
        }
    }

    fn truncate_id(&self, id: &str) -> String {
        if id.len() > 10 {
            format!("{}...{}", &id[..8], &id[id.len()-4..])
        } else {
            id.to_string()
        }
    }

    fn get_transaction_color(&self, transaction: &Transaction) -> String {
        let amount_sui = transaction.amount as f64 / 1_000_000_000.0;
        if amount_sui > 10.0 {
            "\x1b[33m" // Yellow for large transactions
        } else if amount_sui > 1.0 {
            "\x1b[32m" // Green for medium transactions
        } else if amount_sui > 0.1 {
            "\x1b[36m" // Cyan for small transactions
        } else {
            "\x1b[37m" // White for very small transactions
        }.to_string()
    }

    fn repeat_char(&self, c: char, count: usize) -> String {
        std::iter::repeat(c).take(count).collect()
    }

    // Additional utility methods
    pub fn format_welcome_message(&self) -> String {
        let mut message = String::new();
        if self.use_colors {
            message.push_str("\x1b[1;32m"); // Bright green
        }
        message.push_str("SUI Token Transfer Tracker\n");
        if self.use_colors {
            message.push_str("\x1b[0m"); // Reset
        }
        message.push_str("Real-time monitoring of SUI blockchain transfers\n");
        message
    }

    pub fn format_error(&self, error: &str) -> String {
        if self.use_colors {
            format!("\x1b[31mERROR: {}\x1b[0m", error)
        } else {
            format!("ERROR: {}", error)
        }
    }

    pub fn format_warning(&self, warning: &str) -> String {
        if self.use_colors {
            format!("\x1b[33mWARNING: {}\x1b[0m", warning)
        } else {
            format!("WARNING: {}", warning)
        }
    }

    pub fn format_success(&self, message: &str) -> String {
        if self.use_colors {
            format!("\x1b[32m✓ {}\x1b[0m", message)
        } else {
            format!("✓ {}", message)
        }
    }

    pub fn format_info(&self, message: &str) -> String {
        if self.use_colors {
            format!("\x1b[36mℹ {}\x1b[0m", message)
        } else {
            format!("ℹ {}", message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_formatter_creation() {
        let formatter = OutputFormatter::new(true, true);
        assert!(formatter.use_colors);
        assert!(formatter.show_timestamps);
    }

    #[test]
    fn test_format_amount() {
        let formatter = OutputFormatter::new(false, false);
        assert_eq!(formatter.format_amount(1000000000), "1.000000000 SUI");
        assert_eq!(formatter.format_amount(500000000), "0.500000000 SUI");
    }

    #[test]
    fn test_truncate_address() {
        let formatter = OutputFormatter::new(false, false);
        let address = "0x1234567890abcdef1234567890abcdef12345678";
        assert_eq!(formatter.truncate_address(address), "0x123456...45678");
        assert_eq!(formatter.truncate_address("0x1234"), "0x1234");
    }

    #[test]
    fn test_format_status() {
        let formatter = OutputFormatter::new(false, false);
        assert_eq!(formatter.format_status(&crate::transaction_processor::TransactionStatus::Success), "✓");
        assert_eq!(formatter.format_status(&crate::transaction_processor::TransactionStatus::Failed), "✗");
        assert_eq!(formatter.format_status(&crate::transaction_processor::TransactionStatus::Pending), "⏳");
    }

    #[test]
    fn test_format_token_type() {
        let formatter = OutputFormatter::new(false, false);
        assert_eq!(formatter.format_token_type("0x2::sui::SUI"), "SUI");
        assert_eq!(formatter.format_token_type("0x123::my_token::TOKEN"), "TOKEN");
        assert_eq!(formatter.format_token_type("simple_token"), "simple_token");
    }

    #[test]
    fn test_welcome_message() {
        let formatter = OutputFormatter::new(true, true);
        let message = formatter.format_welcome_message();
        assert!(message.contains("SUI Token Transfer Tracker"));
        assert!(message.contains("Real-time monitoring"));
    }

    #[test]
    fn test_error_formatting() {
        let formatter = OutputFormatter::new(true, true);
        let error_msg = formatter.format_error("Test error");
        assert!(error_msg.contains("ERROR: Test error"));
        
        let formatter_no_color = OutputFormatter::new(false, false);
        let error_msg_no_color = formatter_no_color.format_error("Test error");
        assert_eq!(error_msg_no_color, "ERROR: Test error");
    }

    #[test]
    fn test_success_formatting() {
        let formatter = OutputFormatter::new(true, true);
        let success_msg = formatter.format_success("Operation completed");
        assert!(success_msg.contains("✓ Operation completed"));
    }
}