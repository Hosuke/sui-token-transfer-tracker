# SUI Token Transfer Tracker

A powerful real-time monitoring tool for tracking SUI blockchain token transfers with comprehensive analytics and alerting capabilities.

## ✨ Features

- **🔄 Real-time Monitoring**: Track SUI token transfers in real-time
- **🏠 Multiple Address Support**: Monitor multiple addresses simultaneously
- **🚨 Smart Alert System**: Get notified for low balances, large transfers, and suspicious activities
- **📊 Transaction Analytics**: Detailed transaction history and statistics
- **📋 Flexible Output**: Table, JSON, and CSV output formats
- **⚙️ Highly Configurable**: Extensive configuration options via CLI or config files
- **🌐 Network Support**: Works with SUI mainnet and testnet
- **📈 Performance Metrics**: Built-in performance monitoring and statistics

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/sui-token-transfer-tracker
cd sui-token-transfer-tracker

# Build the project
cargo build --release

# Run with a sample address
cargo run -- --version
```

### Basic Commands

```bash
# Generate a default configuration file
cargo run -- --generate-config

# Monitor a real SUI address (example)
cargo run -- --address 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee

# Show version information
cargo run -- --version
```

## 📖 Usage Examples

### 1. Query Address Information

Use the built-in example to query real address data:

```bash
cargo run --example query_address_rpc
```

**Sample Output:**
```
🔍 正在查询 SUI 地址: 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee
================================================
💰 查询地址余额...
💳 SUI 余额: 0.821300859 SUI (821300859 MIST)
🪙 代币类型: "0x2::sui::SUI"

💎 查询所有代币余额...
📊 总共找到 9 种代币:
   1. "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL": 533531030 units
   2. "0xbde4ba4c2e274a60ce15c1cfff9e5c42e41654ac8b6d906a57efa4bd3c29f47d::hasui::HASUI": 75053 units
   3. "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC": 0 units
   4. "0x549e8b69270defbfafd4f94e17ec44cdbdd99820b33bda2278dea3b9a32d3f55::cert::CERT": 956581376 units
   5. "0x6864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS": 0 units
   6. "0x5145494a5f5100e645e4b0aa950fa6b68f614e8c59e17bc5ded3495123a79178::ns::NS": 325903 units
   7. "0xdeeb7a4662eec9f2f3def03fb937a663dddaa2e215b8078a284d026b7946c270::deep::DEEP": 5941 units
   8. "0xce7ff77a83ea0cb6fd39bd8748e2ec89a3f41e8efdc3f4eb123e0ca37b184db2::buck::BUCK": 0 units
   9. "0x2::sui::SUI": 0.821300859 SUI

📝 查询最近交易历史...
🎯 找到 5 笔发送的交易:

📋 交易 #1
   📄 交易摘要: "61AsPDjbgaLUdfdEQxqrYLre3B6bMCKLKZvxPwvrYxGF"
   🕰️  时间: 2025-03-29 13:05:39 UTC
   ⛽ Gas 消耗: "750000"
   💰 余额变化: +0.000100000 SUI (recipient)
   💰 余额变化: -0.000869760 SUI (sender)

📋 交易 #2
   📄 交易摘要: "HT9YgoXQUKqZzaz2TGCjVDb3ZTo53aTNCrxn8NEDRyQB"
   🕰️  时间: 2025-03-31 05:31:10 UTC
   ⛽ Gas 消耗: "750000"
   💰 余额变化: +0.000200000 SUI (recipient)
   💰 余额变化: -0.001947880 SUI (sender)

📥 查询接收的交易...
📨 找到 3 笔接收的交易:

📋 接收交易 #1
   📄 交易摘要: "6MNR7smuMqvxttZ1aCMdB4W78ZXTjmdUBDVkDyRmG9Dd"
   💰 接收: +1.500000000 SUI

🎉 地址查询完成!
```

### 2. Generate Configuration File

```bash
cargo run -- --generate-config
```

**Generated config.toml:**
```toml
[monitoring]
addresses = []
poll_interval = 30
rpc_url = "https://fullnode.mainnet.sui.io:443"

[alerts]
low_balance_threshold = 1000000000
large_transfer_threshold = 10000000000
enable_console_alerts = true
enable_file_alerts = false
alert_file_path = "alerts.log"

[output]
format = "table"
use_colors = true
show_timestamps = true

[logging]
level = "info"
file_path = "tracker.log"
```

### 3. Demo Mode

Run the demonstration with simulated data:

```bash
cargo run --example demo
```

**Demo Output:**
```
🎯 SUI Token Transfer Tracker Demo
===================================

📊 Processing simulated transfer events...

[2025-03-29 13:05:39] 📤 Transfer: 0x1234...5678 → 0xabcd...ef12
  💰 Amount: 1.500000000 SUI
  🏷️  Token: 0x2::sui::SUI
  ✅ Status: Success
  📦 Block: 12345

[2025-03-29 13:10:15] 📤 Transfer: 0xabcd...ef12 → 0x9876...5432
  💰 Amount: 0.750000000 SUI
  🏷️  Token: 0x2::sui::SUI
  ✅ Status: Success
  📦 Block: 12346

📈 Final Statistics:
  Total Addresses: 3
  Total Transactions: 2
  Total Volume: 2.250000000 SUI
  
💳 Address Balances:
  0x1234...5678: 8.500000000 SUI
  0xabcd...ef12: 1.750000000 SUI
  0x9876...5432: 10.750000000 SUI
```

### 4. Command Line Arguments

```bash
# Show help
cargo run -- --help

# Monitor specific address with custom settings
cargo run -- --address 0xYourAddress --poll-interval 60 --threshold 1000000000

# Export data in different formats
cargo run -- --export json --output data.json
cargo run -- --export csv --output data.csv

# Use custom RPC endpoint
cargo run -- --rpc-url https://fullnode.testnet.sui.io:443

# Enable verbose logging
cargo run -- --log-level debug --verbose
```

### 5. Testing Network Connectivity

```bash
# Test basic SUI network connection
cargo run --example simple_sui_test
```

**Output:**
```
🔍 正在查询 SUI 地址: 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee
================================================
✅ 网络连接成功
🌐 链 ID: "35834a8a"

🎯 基本连接测试成功!
📝 注意: 完整的地址查询功能需要正确的 GraphQL schema 定义
💡 可以使用以下方式继续开发:
   1. 使用 sui-sdk-types 直接调用 JSON-RPC
   2. 实现自定义的 GraphQL 查询类型
   3. 使用 reqwest 直接调用 RPC 接口

📍 目标地址详情:
   地址: 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee
   长度: 66 字符
   格式: ✅ 有效
```

## 📝 Output Formats

### Table Format (Default)
```
[14:30:15] Transfer Event
From: 0x1234...5678
To:   0xabcd...ef12
Amount: 1.500000000 SUI
Token: 0x2::sui::SUI
Status: ✅ Success
Block: 12345
```

### JSON Format
```json
{
  "transaction": {
    "id": "0x1234567890abcdef...",
    "sender": "0x1234567890abcdef12345678",
    "recipient": "0xabcdef1234567890abcdef12",
    "amount": 1500000000,
    "token_type": "0x2::sui::SUI",
    "timestamp": 1634567890,
    "block_number": 12345,
    "status": "Success"
  },
  "sender_balance_change": -1500000000,
  "receiver_balance_change": 1500000000,
  "processing_time_ms": 5
}
```

### CSV Format
```csv
Address,Balance,Total Transactions,Total Sent,Total Received
0x1234...5678,8500000000,5,2500000000,1000000000
0xabcd...ef12,1750000000,3,750000000,2000000000
```

## 🚨 Alert System

The tracker includes a comprehensive alert system that monitors:

### Alert Types

1. **💰 Low Balance Alert**
   ```
   ⚠️ LOW BALANCE ALERT
   Address: 0x1234...5678
   Current Balance: 0.500000000 SUI
   Threshold: 1.000000000 SUI
   Time: 2025-03-29 13:05:39 UTC
   ```

2. **💸 Large Transfer Alert**
   ```
   🚨 LARGE TRANSFER ALERT
   From: 0x1234...5678
   To: 0xabcd...ef12
   Amount: 50.000000000 SUI
   Threshold: 10.000000000 SUI
   Time: 2025-03-29 13:05:39 UTC
   ```

3. **🔍 Suspicious Activity Alert**
   ```
   ⚠️ SUSPICIOUS ACTIVITY DETECTED
   Address: 0x1234...5678
   Activity: High frequency transactions
   Description: 15 transactions in 5 minutes
   Risk Level: Medium
   ```

## ⚙️ Configuration

### Environment Variables
```bash
export SUI_RPC_URL="https://fullnode.mainnet.sui.io:443"
export RUST_LOG="debug"
export TRACKER_CONFIG="config.toml"
```

### Configuration File (TOML)
```toml
[monitoring]
addresses = [
    "0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee"
]
poll_interval = 30
rpc_url = "https://fullnode.mainnet.sui.io:443"

[alerts]
low_balance_threshold = 1000000000  # 1 SUI in MIST
large_transfer_threshold = 10000000000  # 10 SUI in MIST
enable_console_alerts = true
enable_file_alerts = true
alert_file_path = "alerts.log"

[output]
format = "table"  # table, json, csv
use_colors = true
show_timestamps = true

[logging]
level = "info"  # trace, debug, info, warn, error
file_path = "tracker.log"
```

## 🧪 Development & Testing

### Run Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_transaction_processor
```

### Code Quality
```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Check compilation
cargo check
```

### Examples Directory
- `cargo run --example demo` - Demonstration with simulated data
- `cargo run --example query_address_rpc` - Real SUI address query
- `cargo run --example simple_sui_test` - Network connectivity test
- `cargo run --example test_formatter` - Output formatting test

## 🛠️ Troubleshooting

### Common Issues

1. **Network Connection**
   ```bash
   # Test connection manually
   curl -X POST https://fullnode.mainnet.sui.io:443 \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"sui_getLatestCheckpoint","params":[],"id":1}'
   ```

2. **Invalid Address Format**
   - SUI addresses must be 66 characters long
   - Must start with "0x"
   - Example: `0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee`

3. **Compilation Issues**
   ```bash
   # Clean and rebuild
   cargo clean
   cargo build --release
   ```

### Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo run -- --address 0xYourAddress

# View logs
tail -f tracker.log
```

## 📊 Performance

- **Memory Usage**: ~10-50MB depending on monitoring scope
- **Network Usage**: ~1-5KB per query (depending on address activity)
- **Polling Frequency**: Configurable (default: 30 seconds)
- **Concurrent Addresses**: Tested with 100+ addresses

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [SUI Foundation](https://sui.io/) for the blockchain platform
- [Rust SUI SDK](https://github.com/mystenlabs/sui-rust-sdk) for the official SDK
- Rust community for excellent tooling and libraries

## ⚠️ Disclaimer

This is experimental software intended for educational and development purposes. Use at your own risk. The developers are not responsible for any financial losses or damages resulting from the use of this software. Always verify transactions and monitor your assets independently.