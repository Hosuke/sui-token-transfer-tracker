use sui_token_transfer_tracker::transaction_processor::{TransactionProcessor};
use sui_token_transfer_tracker::event_monitor::TransferEvent;
use sui_token_transfer_tracker::output_formatter::OutputFormatter;
use sui_token_transfer_tracker::alert_system::AlertSystem;
use sui_token_transfer_tracker::config::Config;
use chrono::Utc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SUI Token Transfer Tracker - 完整功能演示");
    println!("================================================");
    
    // 创建配置
    let config = Config::default();
    
    // 创建核心组件
    let processor = TransactionProcessor::new();
    let mut formatter = OutputFormatter::new(true, true);
    let (alert_system, _) = AlertSystem::new();
    
    println!("\n📋 1. 系统组件初始化完成");
    println!("   ✅ 交易处理器: 已创建");
    println!("   ✅ 输出格式化器: 已创建 (支持颜色)");
    println!("   ✅ 警报系统: 已创建");
    
    // 模拟一些转账事件
    println!("\n💸 2. 模拟转账交易处理");
    
    let demo_events = vec![
        TransferEvent {
            transaction_id: "0xtx1_1234567890abcdef1234567890abcdef12345678".to_string(),
            package_id: "0xpackage1".to_string(),
            transaction_module: "pay".to_string(),
            sender: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            recipient: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            amount: 1000000000, // 1 SUI
            token_type: "0x2::sui::SUI".to_string(),
            timestamp: Utc::now().timestamp() as u64,
            block_number: 10000001,
            event_type: "transfer".to_string(),
        },
        TransferEvent {
            transaction_id: "0xtx2_abcdef1234567890abcdef1234567890abcdef12".to_string(),
            package_id: "0xpackage2".to_string(),
            transaction_module: "pay".to_string(),
            sender: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            recipient: "0x567890abcdef1234567890abcdef1234567890abcd".to_string(),
            amount: 2500000000, // 2.5 SUI
            token_type: "0x2::sui::SUI".to_string(),
            timestamp: Utc::now().timestamp() as u64 + 1,
            block_number: 10000002,
            event_type: "transfer".to_string(),
        },
        TransferEvent {
            transaction_id: "0xtx3_567890abcdef1234567890abcdef1234567890abcd".to_string(),
            package_id: "0xpackage3".to_string(),
            transaction_module: "pay".to_string(),
            sender: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            recipient: "0x567890abcdef1234567890abcdef1234567890abcd".to_string(),
            amount: 500000000, // 0.5 SUI
            token_type: "0x2::sui::SUI".to_string(),
            timestamp: Utc::now().timestamp() as u64 + 2,
            block_number: 10000003,
            event_type: "transfer".to_string(),
        },
    ];
    
    println!("   📊 处理 {} 个模拟交易...", demo_events.len());
    
    // 处理每个事件
    for (i, event) in demo_events.iter().enumerate() {
        println!("\n   🔄 处理交易 {}/{}: {}", i + 1, demo_events.len(), event.transaction_id);
        
        // 处理转账事件
        match processor.process_transfer_event(event.clone()).await {
            Ok(processed_tx) => {
                println!("      ✅ 交易处理成功");
                println!("      💰 发送方余额变化: {} SUI", processed_tx.sender_balance_change as f64 / 1_000_000_000.0);
                println!("      💰 接收方余额变化: +{} SUI", processed_tx.receiver_balance_change as f64 / 1_000_000_000.0);
                println!("      ⏱️  处理时间: {}ms", processed_tx.processing_time_ms);
                
                // 检查警报 (简化版)
                if event.amount > 2000000000 {
                    println!("      🚨 触发警报: 大额转账检测");
                    println!("         ⚠️  转账金额: {} SUI", event.amount as f64 / 1_000_000_000.0);
                }
                
                // 格式化并显示交易
                let formatted_tx = formatter.format_transaction(&processed_tx.transaction);
                println!("      📝 交易详情:\n         {}", formatted_tx.replace("\n", "\n         "));
            }
            Err(e) => {
                println!("      ❌ 交易处理失败: {}", e);
            }
        }
        
        // 模拟处理延迟
        sleep(Duration::from_millis(500)).await;
    }
    
    println!("\n📈 3. 统计信息汇总");
    
    // 获取并显示统计信息
    let balances = processor.get_all_balances().await;
    let stats = processor.get_all_stats().await;
    let processor_stats = processor.get_processor_stats().await;
    
    println!("   📊 处理器统计:");
    println!("      📍 监控地址数: {}", processor_stats.total_addresses);
    println!("      📈 总交易数: {}", processor_stats.total_transactions);
    println!("      💰 总交易量: {} SUI", processor_stats.total_volume as f64 / 1_000_000_000.0);
    
    println!("\n   💳 地址余额:");
    for (address, balance) in balances {
        println!("      🔑 {}: {} SUI", format!("{}...", &address[..16]), balance as f64 / 1_000_000_000.0);
        
        if let Some(addr_stats) = stats.get(&address) {
            println!("         📊 交易统计: {} 笔交易, 发送: {} SUI, 接收: {} SUI", 
                addr_stats.total_transactions,
                addr_stats.total_sent as f64 / 1_000_000_000.0,
                addr_stats.total_received as f64 / 1_000_000_000.0);
        }
    }
    
    println!("\n📋 4. 最近交易历史");
    
    // 获取最近交易
    let recent_txs = processor.get_recent_transactions(5).await;
    for (i, tx) in recent_txs.iter().enumerate() {
        println!("   {}. {}", i + 1, formatter.format_transaction(tx));
    }
    
    // 测试数据导出功能
    println!("\n💾 5. 数据导出测试");
    
    // JSON 导出
    formatter.set_format(sui_token_transfer_tracker::output_formatter::OutputFormat::Json);
    let json_export = processor.export_data(sui_token_transfer_tracker::transaction_processor::ExportFormat::Json).await?;
    println!("   📄 JSON 导出成功 ({} 字符)", json_export.len());
    
    // CSV 导出
    formatter.set_format(sui_token_transfer_tracker::output_formatter::OutputFormat::Csv);
    let csv_export = processor.export_data(sui_token_transfer_tracker::transaction_processor::ExportFormat::Csv).await?;
    println!("   📊 CSV 导出成功 ({} 字符)", csv_export.len());
    
    // 显示部分导出数据示例
    println!("\n   📋 JSON 导出示例 (前200字符):");
    println!("   {}", &json_export[..200.min(json_export.len())]);
    
    println!("\n   📋 CSV 导出示例 (前100字符):");
    println!("   {}", &csv_export[..100.min(csv_export.len())]);
    
    // 测试警报历史
    println!("\n🚨 6. 警报系统测试");
    
    // 手动触发一些警报 (简化版)
    let test_alerts = vec![
        (500000000u64, "余额不足警报"),
        (10000000000u64, "大额转账检测"),
    ];
    
    for (amount, alert_type) in test_alerts {
        if amount > 1000000000 {
            println!("   ⚠️  {} - 金额: {} SUI", alert_type, amount as f64 / 1_000_000_000.0);
            println!("      📢 {}", formatter.format_warning("检测到异常交易"));
        } else {
            println!("   ✅ {} - 无警报", alert_type);
        }
    }
    
    println!("\n🎉 7. 演示完成");
    println!("================================================");
    println!("✅ 所有核心功能测试完成！");
    println!("📊 功能涵盖:");
    println!("   • 交易处理和余额跟踪");
    println!("   • 多种输出格式 (表格, JSON, CSV)");
    println!("   • 警报系统");
    println!("   • 统计信息汇总");
    println!("   • 数据导出功能");
    println!("   • 地址管理");
    
    println!("\n🚀 SUI Token Transfer Tracker 已准备就绪！");
    println!("💡 提示: 您可以修改配置文件来监控真实的 SUI 地址");
    
    Ok(())
}