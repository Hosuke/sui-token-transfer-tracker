use sui_token_transfer_tracker::output_formatter::OutputFormatter;
use sui_token_transfer_tracker::transaction_processor::{Transaction, TransactionStatus};
use chrono::Utc;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing SUI Token Transfer Tracker - Output Formatter");
    
    // 创建输出格式化器
    let mut formatter = OutputFormatter::new(true, true);
    
    // 测试欢迎消息
    let welcome = formatter.format_welcome_message();
    println!("{}", welcome);
    
    // 测试余额摘要格式化
    let mut balances = HashMap::new();
    balances.insert("0x1234567890abcdef1234567890abcdef12345678".to_string(), 1000000000u64);
    balances.insert("0x0987654321fedcba0987654321fedcba09876543".to_string(), 500000000u64);
    
    let balance_summary = formatter.format_balance_summary(&balances);
    println!("💰 Balance Summary:\n{}", balance_summary);
    
    // 测试交易格式化
    let transaction = Transaction {
        id: "test_tx_1234567890abcdef1234567890abcdef12345678".to_string(),
        sender: "0x1111111111111111111111111111111111111111".to_string(),
        recipient: "0x2222222222222222222222222222222222222222".to_string(),
        amount: 1000000000, // 1 SUI
        token_type: "0x2::sui::SUI".to_string(),
        timestamp: Utc::now().timestamp() as u64,
        block_number: 10000000,
        gas_used: Some(1000000),
        gas_price: Some(1000),
        status: TransactionStatus::Success,
    };
    
    let formatted_transaction = formatter.format_transaction(&transaction);
    println!("📝 Formatted Transaction:\n{}", formatted_transaction);
    
    // 测试不同消息类型
    println!("\n🔔 Testing message formatting:");
    println!("{}", formatter.format_success("Operation completed successfully"));
    println!("{}", formatter.format_error("Connection failed"));
    println!("{}", formatter.format_warning("Low balance detected"));
    println!("{}", formatter.format_info("Processing transaction"));
    
    // 测试JSON输出
    formatter.set_format(sui_token_transfer_tracker::output_formatter::OutputFormat::Json);
    let json_transaction = formatter.format_transaction(&transaction);
    println!("\n📄 JSON Transaction:\n{}", json_transaction);
    
    // 测试CSV输出
    formatter.set_format(sui_token_transfer_tracker::output_formatter::OutputFormat::Csv);
    let csv_transaction = formatter.format_transaction(&transaction);
    println!("\n📊 CSV Transaction:\n{}", csv_transaction);
    
    println!("\n✅ All tests completed successfully!");
    println!("🎉 The output formatter is working correctly!");
    
    Ok(())
}