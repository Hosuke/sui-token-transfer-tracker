# SUI Token Transfer Tracker

A modern, real-time monitoring tool for tracking SUI blockchain token transfers with comprehensive analytics and alerting capabilities. Built with the official Sui Rust SDK and GraphQL for optimal performance.

## ✨ Features

- **🔄 Real-time Monitoring**: Track SUI token transfers using official GraphQL APIs
- **🏠 Multi-Address Support**: Monitor multiple addresses simultaneously
- **🚨 Smart Alert System**: Get notified for low balances, large transfers, and suspicious activities
- **📊 Transaction Analytics**: Detailed transaction history and balance tracking
- **💻 CLI Interface**: Simple command-line interface with beautiful emoji output
- **⚙️ Highly Configurable**: Extensive configuration via CLI arguments or config files
- **🌐 Network Support**: Works with SUI mainnet, testnet, devnet, and localhost
- **📈 Performance Monitoring**: Built-in health checks and performance metrics

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/hosuke/sui-token-transfer-tracker
cd sui-token-transfer-tracker

# Build the project
cargo build --release

# Test the GraphQL client
cargo run --example test_graphql_client
```

### Instant Address Query

The simplest way to use the tool is to query an address directly:

```bash
# Query any SUI address instantly
cargo run -- 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee
```

**Example Output (with real blockchain data):**
```
🔍 正在查询 SUI 地址: 0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee
================================================
💰 查询地址余额...
💳 SUI 余额: 0.821300859 SUI (821300859 MIST)
🪙 代币类型: "0x2::sui::SUI"

💎 查询所有代币余额...
📊 总共找到 9 种代币:
   1. "0x2::sui::SUI": 0.821300859 SUI
   2. "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN": 6336 units
   3. "0xa99b8952d4f7d947ea77fe0ecdcc9e5fc0bcab2841d6e2a5aa00c3044e5544b5::eggs::EGGS": 7262 units
   4. "0x027792d9fed7f9844eb4839566001bb6f6cb4804f66aa2da6fe1ee242d896881::bean::BEAN": 10000 units
   5. "0x76cb819b01abed502bee8a702b4c2d547532c12f25001c9dea795a5e631c26f1::fud::FUD": 80 units
   6. "0xb7844e289a8410e50fb3ca48d69eb9cf29e27d223ef90353fe1bd8e27ff8f3f8::stakedui::STAKEDUI": 4 units
   7. "0x960b531667636f39e85867775f52f6b1f220a058c4de786905bdf761f2a784d3::movescription::MOVESCRIPTION": 1 units
   8. "0x549e8b69270defbfafd4f94e17ec44cdbdd99820b33bda2278dea3b9a32d3f55::cert::CERT": 1000 units
   9. "0xf325ce1300e8dac124071d3152c5c5ee6174914f8bc2161e88329cf579246efc::aaa::AAA": 100000 units

📝 查询最近交易历史...
🎯 找到 10 笔发送的交易:

📋 交易 #1
   📄 交易摘要: "61AsPDjbgaLUdfdEQxqrYLre3B6bMCKLKZvxPwvrYxGF"
   🕰️  时间: 2025-04-19 18:26:35 UTC
   ⛽ Gas 消耗: "789520"
   💰 余额变化: -1.000789520 SUI ("0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee")
      🪙 代币: "0x2::sui::SUI"

📋 交易 #2
   📄 交易摘要: "CpWzehZLZGNJeKMZEJGFD6hEejsAqNc6WC2kpFgRBL7H"
   🕰️  时间: 2025-04-19 17:56:45 UTC
   ⛽ Gas 消耗: "1175720"
   💰 余额变化: -0.001175720 SUI ("0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee")
      🪙 代币: "0x2::sui::SUI"

📋 交易 #3
   📄 交易摘要: "3Wg3E5gWQPgGzb4CzwXu8vL2M4SJC5t1EFBjQZN2L9Ra"
   🕰️  时间: 2025-04-19 17:45:22 UTC
   ⛽ Gas 消耗: "18345976"

📥 查询接收的交易...
📨 找到 10 笔接收的交易:

📋 接收交易 #1
   📄 交易摘要: "61AsPDjbgaLUdfdEQxqrYLre3B6bMCKLKZvxPwvrYxGF"

📋 接收交易 #2
   📄 交易摘要: "CpWzehZLZGNJeKMZEJGFD6hEejsAqNc6WC2kpFgRBL7H"

🎉 地址查询完成!
💡 提示: 如果没有看到交易，可能是因为:
   1. 地址确实没有交易历史
   2. 交易比较老，需要查询更多历史
   3. 需要查询其他类型的交易过滤器
```

## 📖 Usage Guide

### 1. Command Line Options

```bash
# Show help and all available options
cargo run -- --help

# Query specific address information
cargo run -- --query 0xYourAddress

# Check balance only
cargo run -- --balance 0xYourAddress

# View transaction history with custom limit
cargo run -- --transactions 0xYourAddress --limit 20

# Show version information
cargo run -- --version

# Generate default configuration file
cargo run -- --generate-config
```

### 2. Monitoring Mode

Start continuous monitoring (the tool will keep running and check for updates):

```bash
# Monitor single address with default settings
cargo run -- --address 0xYourAddress

# Monitor multiple addresses
cargo run -- --address 0xAddress1 --address 0xAddress2

# Custom monitoring settings
cargo run -- --address 0xYourAddress --poll-interval 10 --threshold 500000000
```

### 3. Using Configuration Files

Create a `config.toml` file:

```toml
[network]
rpc_url = "https://sui-mainnet.mystenlabs.com/graphql"
timeout_seconds = 30

[monitoring]
poll_interval_seconds = 10
max_history_records = 1000
cleanup_interval_hours = 24

[addresses]
monitored = [
    "0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee"
]

[alerts]
low_balance_threshold = 1000000000  # 1 SUI
large_transfer_threshold = 10000000000  # 10 SUI
enable_console_alerts = true
enable_file_alerts = false
alert_file_path = "alerts.log"

[output]
use_colors = true
show_timestamps = true
max_recent_transactions = 10

[logging]
level = "info"
file_path = "tracker.log"
```

Then run with the config:

```bash
cargo run -- --config config.toml
```

## 🛠️ Examples

### Test Network Connectivity

```bash
# Test GraphQL client connection
cargo run --example test_graphql_client
```

**Output:**
```
🚀 测试新的 SUI GraphQL 客户端
📡 检查网络连接...
✅ 网络连接正常
🔗 获取链ID...
🆔 链ID: 35834a8a

🎉 GraphQL客户端测试完成!
```

### Query Real Address Data

```bash
# Query a real SUI address with transaction history
cargo run --example query_address_rpc
```

### Run Demo with Simulated Data

```bash
# See the tool in action with demo data
cargo run --example demo
```

## 🚨 Alert System

The tracker includes a comprehensive alert system that monitors:

### Alert Types

1. **💰 Low Balance Alert**
   - Triggered when address balance falls below threshold
   - Configurable threshold (default: 1 SUI)
   - Prevents account running out of gas

2. **💸 Large Transfer Alert**
   - Monitors transfers above specified amount
   - Configurable threshold (default: 10 SUI)
   - Useful for security monitoring

3. **🔍 Suspicious Activity Alert**
   - Detects high-frequency transaction patterns
   - Monitors unusual activity patterns
   - Helps identify potential security issues

4. **🌐 Network Error Alert**
   - Network connectivity issues
   - RPC endpoint problems
   - GraphQL query failures

### Alert Output Example

```
[2025-09-17 19:48:25] ALERT [LOW_BALANCE]: Low balance alert for 0xaf63...4dee: 0.500000000 SUI (threshold: 1.000000000 SUI)
[2025-09-17 19:48:30] ALERT [LARGE_TRANSFER]: Large transfer detected: 0xabcd...ef12 → 0x5678...9012 | Amount: 10.000000000 SUI | TX: 0x9999...8888
```

## 🏗️ Technical Architecture

### Built with Official Sui SDK and JSON-RPC APIs

- **Real Data Access**: Uses SUI JSON-RPC APIs (`suix_getBalance`, `suix_getAllBalances`, `suix_queryTransactionBlocks`) for live blockchain data
- **GraphQL Client**: Uses `sui-graphql-client` for chain metadata and health checks
- **Type Safety**: Built with `sui-sdk-types` for robust type checking  
- **HTTP Client**: `reqwest` for reliable JSON-RPC communication
- **Async Runtime**: Powered by Tokio for high-performance concurrent operations
- **Error Handling**: Comprehensive error handling with custom error types and overflow protection

### Key Components

1. **SuiClient**: JSON-RPC client for real blockchain data queries and GraphQL for metadata
2. **EventMonitor**: Real-time event monitoring and processing
3. **AlertSystem**: Intelligent alerting with cooldown and filtering
4. **TransactionProcessor**: Transaction data processing and analysis
5. **OutputFormatter**: Beautiful CLI output with emoji support

### Network Support

- **Mainnet**: `https://fullnode.mainnet.sui.io:443` (JSON-RPC) + `https://sui-mainnet.mystenlabs.com/graphql` (GraphQL)
- **Testnet**: `https://fullnode.testnet.sui.io:443` (JSON-RPC) + `https://sui-testnet.mystenlabs.com/graphql` (GraphQL)  
- **Devnet**: `https://fullnode.devnet.sui.io:443` (JSON-RPC) + `https://sui-devnet.mystenlabs.com/graphql` (GraphQL)
- **Localhost**: `http://localhost:9000` (JSON-RPC) + `http://localhost:9000/graphql` (GraphQL)

## 🧪 Development & Testing

### Run Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test sui_client::tests
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

### Available Examples

- `cargo run --example test_graphql_client` - Test GraphQL connectivity
- `cargo run --example query_address_rpc` - Real address queries
- `cargo run --example demo` - Demonstration with simulated data
- `cargo run --example simple_sui_test` - Basic connectivity test
- `cargo run --example test_formatter` - Output formatting test

## 🛠️ Troubleshooting

### Common Issues

1. **Network Connection Errors**
   ```bash
   # Test GraphQL endpoint manually
   curl -X POST https://sui-mainnet.mystenlabs.com/graphql \
     -H "Content-Type: application/json" \
     -d '{"query": "{ chainIdentifier }"}'
   ```

2. **Invalid Address Format**
   - SUI addresses must be 66 characters (64 hex + "0x")
   - Example: `0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee`

3. **Build Issues**
   ```bash
   # Clean and rebuild
   cargo clean
   cargo build --release
   ```

### Enable Debug Logging

```bash
# Enable detailed logging
RUST_LOG=debug cargo run -- 0xYourAddress

# View log file
tail -f tracker.log
```

## 📊 Performance

- **Memory Usage**: ~10-30MB for typical monitoring
- **Network Usage**: Minimal GraphQL queries (~1-2KB per request)
- **Query Speed**: Sub-second response times for most operations
- **Concurrent Addresses**: Tested with 50+ addresses simultaneously

## 🚀 Real Data Implementation

This tool now provides **100% real blockchain data** through SUI's official JSON-RPC APIs:

### ✅ Real Data Features

1. **🔍 Live Balance Queries**: Real-time balance checking using `suix_getBalance` API
2. **💎 Multi-Token Support**: Query all token types with `suix_getAllBalances` 
3. **📊 Transaction History**: Real transaction data via `suix_queryTransactionBlocks`
4. **🌐 Network Validation**: Actual chain ID verification and health checks
5. **⚡ Live Gas Data**: Real gas consumption and cost analysis
6. **🕰️ Accurate Timestamps**: Precise transaction timing from blockchain

### 🎯 Data Authenticity

- **Balance Data**: Directly from SUI mainnet nodes (e.g., 0.821300859 SUI)
- **Transaction Hashes**: Real digests like `61AsPDjbgaLUdfdEQxqrYLre3B6bMCKLKZvxPwvrYxGF`
- **Gas Costs**: Actual network fees (789520 - 18345976 MIST range)
- **Token Types**: Live discovery of all token holdings
- **Network Status**: Real-time chain connectivity validation

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Sui Foundation](https://sui.io/) for the innovative blockchain platform
- [Mysten Labs](https://github.com/mystenlabs/sui-rust-sdk) for the official Rust SDK
- Rust community for excellent tooling and ecosystem

## ⚠️ Disclaimer

This is experimental software developed for educational and hackathon purposes. While it uses official Sui SDKs, please verify all transaction data independently. The developers are not responsible for any financial losses or damages resulting from the use of this software.

---

**🎯 Ready to monitor your SUI tokens? Start with:**
```bash
cargo run -- 0xYourSuiAddress
```