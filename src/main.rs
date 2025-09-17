use clap::{Arg, ArgMatches, Command};
use sui_token_transfer_tracker::{TokenTransferTracker, Config, config::ConfigArgs, TrackerResult, TrackerError, OutputFormat};
use std::path::Path;

#[tokio::main]
async fn main() -> TrackerResult<()> {
    // 解析命令行参数
    let matches = parse_args();

    // 初始化日志
    if let Err(_) = env_logger::try_init_from_env(env_logger::Env::default().default_filter_or("info")) {
        // 日志已经初始化过了，忽略错误
    }

    // 处理不需要网络连接的简单命令
    if handle_simple_commands(&matches).await? {
        return Ok(());
    }

    // 加载配置
    let config = load_config(&matches).await?;

    // 创建跟踪器
    let mut tracker = TokenTransferTracker::new(config).await?;

    // 处理需要跟踪器的命令
    handle_tracker_commands(&matches, &mut tracker).await?;

    // 启动监控（如果需要）
    if should_start_monitoring(&matches) {
        println!("{}", tracker.output_formatter.format_welcome_message());
        
        // 简化实现：直接运行监控，用户可以用Ctrl+C停止
        if let Err(e) = tracker.start_monitoring().await {
            eprintln!("Error starting monitoring: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn parse_args() -> ArgMatches {
    Command::new("SUI Token Transfer Tracker")
        .version("0.1.0")
        .about("Real-time monitoring of SUI token transfers")
        .author("Your Name <your.email@example.com>")
        .disable_version_flag(true)  // 禁用自动生成的版本标志
        
        // 配置文件选项
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Path to configuration file")
            .num_args(1))
        
        // 地址管理选项
        .arg(Arg::new("address")
            .short('a')
            .long("address")
            .value_name("ADDRESS")
            .help("SUI address to monitor (can be used multiple times)")
            .num_args(1)
            .action(clap::ArgAction::Append))
        
        .arg(Arg::new("add-address")
            .long("add-address")
            .value_name("ADDRESS")
            .help("Add address to monitoring list")
            .num_args(1))
        
        .arg(Arg::new("remove-address")
            .long("remove-address")
            .value_name("ADDRESS")
            .help("Remove address from monitoring list")
            .num_args(1))
        
        .arg(Arg::new("list-addresses")
            .long("list-addresses")
            .help("List all monitored addresses")
            .action(clap::ArgAction::SetTrue))
        
        // 网络配置
        .arg(Arg::new("rpc-url")
            .long("rpc-url")
            .value_name("URL")
            .help("SUI network RPC URL")
            .num_args(1))
        
        .arg(Arg::new("poll-interval")
            .short('i')
            .long("poll-interval")
            .value_name("SECONDS")
            .help("Polling interval in seconds")
            .num_args(1))
        
        // 警报配置
        .arg(Arg::new("threshold")
            .short('t')
            .long("threshold")
            .value_name("AMOUNT")
            .help("Low balance threshold in SUI")
            .num_args(1))
        
        .arg(Arg::new("large-transfer-threshold")
            .long("large-transfer-threshold")
            .value_name("AMOUNT")
            .help("Large transfer threshold in SUI")
            .num_args(1))
        
        // 输出选项
        .arg(Arg::new("no-colors")
            .long("no-colors")
            .help("Disable colored output")
            .action(clap::ArgAction::SetTrue))
        
        .arg(Arg::new("no-timestamps")
            .long("no-timestamps")
            .help("Disable timestamps in output")
            .action(clap::ArgAction::SetTrue))
        
        .arg(Arg::new("output-format")
            .long("output-format")
            .value_name("FORMAT")
            .help("Output format (table, json, csv)")
            .num_args(1)
            .value_parser(["table", "json", "csv"]))
        
        // 日志选项
        .arg(Arg::new("log-level")
            .long("log-level")
            .value_name("LEVEL")
            .help("Log level (trace, debug, info, warn, error)")
            .num_args(1)
            .value_parser(["trace", "debug", "info", "warn", "error"]))
        
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Enable verbose output")
            .action(clap::ArgAction::SetTrue))
        
        // 操作选项
        .arg(Arg::new("force-check")
            .long("force-check")
            .help("Force balance check for all addresses")
            .action(clap::ArgAction::SetTrue))
        
        .arg(Arg::new("export")
            .long("export")
            .value_name("FORMAT")
            .help("Export data (json, csv)")
            .num_args(1)
            .value_parser(["json", "csv"]))
        
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("FILE")
            .help("Output file for export")
            .num_args(1))
        
        .arg(Arg::new("generate-config")
            .long("generate-config")
            .help("Generate default configuration file")
            .action(clap::ArgAction::SetTrue))
        
        .arg(Arg::new("dry-run")
            .long("dry-run")
            .help("Run in dry-run mode (no actual monitoring)")
            .action(clap::ArgAction::SetTrue))
        
        .arg(Arg::new("version")
            .short('V')
            .long("version")
            .help("Show version information")
            .action(clap::ArgAction::SetTrue))
        
        // 位置参数
        .arg(Arg::new("addresses")
            .help("Addresses to monitor")
            .action(clap::ArgAction::Append)
            .num_args(0..))
        
        .get_matches()
}

async fn load_config(matches: &ArgMatches) -> TrackerResult<Config> {
    let mut config = Config::load(matches.get_one::<String>("config").map(|s| s.as_str()))?;
    
    // 收集命令行参数
    let mut args = ConfigArgs::default();
    
    // 地址参数
    if let Some(addresses) = matches.get_many::<String>("address") {
        args.addresses.extend(addresses.map(|s| s.to_string()));
    }
    
    if let Some(addresses) = matches.get_many::<String>("addresses") {
        args.addresses.extend(addresses.map(|s| s.to_string()));
    }
    
    // 网络参数
    if let Some(rpc_url) = matches.get_one::<String>("rpc-url") {
        args.rpc_url = Some(rpc_url.to_string());
    }
    
    if let Some(poll_interval) = matches.get_one::<String>("poll-interval") {
        args.poll_interval = Some(poll_interval.parse()
            .map_err(|_| TrackerError::Configuration("Invalid poll interval".to_string()))?);
    }
    
    // 警报参数
    if let Some(threshold) = matches.get_one::<String>("threshold") {
        args.low_balance_threshold = Some(threshold.parse()
            .map_err(|_| TrackerError::Configuration("Invalid threshold".to_string()))?);
    }
    
    if let Some(large_threshold) = matches.get_one::<String>("large-transfer-threshold") {
        args.large_transfer_threshold = Some(large_threshold.parse()
            .map_err(|_| TrackerError::Configuration("Invalid large transfer threshold".to_string()))?);
    }
    
    // 输出参数
    args.use_colors = Some(!matches.get_flag("no-colors"));
    args.show_timestamps = Some(!matches.get_flag("no-timestamps"));
    
    if let Some(log_level) = matches.get_one::<String>("log-level") {
        args.log_level = Some(log_level.to_string());
    }
    
    // 应用命令行参数
    config.merge_with_args(&args);
    
    Ok(config)
}

async fn handle_simple_commands(matches: &ArgMatches) -> TrackerResult<bool> {
    // 显示版本信息
    if matches.get_flag("version") {
        println!("SUI Token Transfer Tracker v0.1.0");
        return Ok(true);
    }
    
    // 生成配置文件
    if matches.get_flag("generate-config") {
        let config_content = Config::generate_default_config();
        let config_path = Path::new("config.toml");
        std::fs::write(config_path, config_content)?;
        println!("Default configuration file generated: config.toml");
        return Ok(true);
    }
    
    Ok(false)
}

async fn handle_tracker_commands(matches: &ArgMatches, tracker: &mut TokenTransferTracker) -> TrackerResult<()> {
    // 添加地址
    if let Some(address) = matches.get_one::<String>("add-address") {
        tracker.add_address(address.to_string()).await?;
        return Ok(());
    }
    
    // 移除地址
    if let Some(address) = matches.get_one::<String>("remove-address") {
        tracker.remove_address(address).await?;
        return Ok(());
    }
    
    // 列出地址
    if matches.get_flag("list-addresses") {
        let addresses = tracker.get_all_addresses().await;
        println!("Monitored addresses:");
        for address in addresses {
            if let Some(info) = tracker.get_address_info(&address).await {
                println!("  {}: {} ({} transactions)", 
                    address, 
                    tracker.output_formatter.format_amount(info.balance),
                    info.total_transactions);
            }
        }
        return Ok(());
    }
    
    // 强制余额检查
    if matches.get_flag("force-check") {
        tracker.force_balance_check().await?;
        return Ok(());
    }
    
    // 导出数据
    if let Some(format) = matches.get_one::<String>("export") {
        let output_path = matches.get_one::<String>("output").map(|s| s.as_str()).unwrap_or("export");
        tracker.export_data(format, output_path).await?;
        return Ok(());
    }
    
    // 设置输出格式
    if let Some(format) = matches.get_one::<String>("output-format") {
        match format.as_str() {
            "table" => tracker.output_formatter.set_format(OutputFormat::Table),
            "json" => tracker.output_formatter.set_format(OutputFormat::Json),
            "csv" => tracker.output_formatter.set_format(OutputFormat::Csv),
            _ => return Err(TrackerError::Configuration("Invalid output format".to_string())),
        }
    }
    
    Ok(())
}

fn should_start_monitoring(matches: &ArgMatches) -> bool {
    // 如果指定了特定的操作命令，不启动监控
    !matches.get_flag("version") &&
    !matches.get_flag("generate-config") &&
    !matches.contains_id("add-address") &&
    !matches.contains_id("remove-address") &&
    !matches.get_flag("list-addresses") &&
    !matches.get_flag("force-check") &&
    !matches.contains_id("export") &&
    !matches.get_flag("dry-run")
}

async fn output_final_stats(tracker: &TokenTransferTracker) -> TrackerResult<()> {
    let stats = tracker.get_tracker_stats().await;
    let processor_stats = tracker.transaction_processor.get_processor_stats().await;
    
    println!("\n=== Final Statistics ===");
    println!("Uptime: {} seconds", stats.uptime_seconds);
    println!("Events processed: {}", stats.total_events_processed);
    println!("Transactions processed: {}", stats.total_transactions_processed);
    println!("Alerts sent: {}", stats.total_alerts_sent);
    println!("Errors encountered: {}", stats.total_errors);
    println!("Addresses monitored: {}", stats.addresses_monitored);
    println!("Total addresses in processor: {}", processor_stats.total_addresses);
    println!("Total volume processed: {}", tracker.output_formatter.format_amount(processor_stats.total_volume));
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_start_monitoring() {
        let app = Command::new("test");
        
        // 测试没有特殊参数时应该启动监控
        let matches = app.try_get_matches_from(&["test"]).unwrap();
        assert!(should_start_monitoring(&matches));
        
        // 测试有版本参数时不应该启动监控
        let app = Command::new("test").arg(Arg::new("version").long("version").action(clap::ArgAction::SetTrue));
        let matches = app.try_get_matches_from(&["test", "--version"]).unwrap();
        assert!(!should_start_monitoring(&matches));
        
        // 测试有生成配置参数时不应该启动监控
        let app = Command::new("test").arg(Arg::new("generate-config").long("generate-config").action(clap::ArgAction::SetTrue));
        let matches = app.try_get_matches_from(&["test", "--generate-config"]).unwrap();
        assert!(!should_start_monitoring(&matches));
    }

    #[tokio::test]
    async fn test_config_loading() {
        // 测试加载默认配置
        let app = Command::new("test");
        let matches = app.try_get_matches_from(&["test"]).unwrap();
        let config = load_config(&matches).await;
        assert!(config.is_ok());
        
        if let Ok(config) = config {
            assert!(!config.network.rpc_url.is_empty());
            assert!(config.monitoring.poll_interval_seconds > 0);
        }
    }
}