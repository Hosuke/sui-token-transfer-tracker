use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let target_address = "0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee";
    
    println!("🔍 正在查询 SUI 地址: {}", target_address);
    println!("================================================");
    
    let client = Client::new();
    let rpc_url = "https://fullnode.mainnet.sui.io:443";
    
    // 1. 查询地址余额
    println!("💰 查询地址余额...");
    let balance_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "suix_getBalance",
        "params": [target_address]
    });
    
    match client.post(rpc_url)
        .json(&balance_request)
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                if let Some(total_balance) = result.get("totalBalance") {
                    if let Some(balance_str) = total_balance.as_str() {
                        if let Ok(balance) = balance_str.parse::<u64>() {
                            let sui_amount = balance as f64 / 1_000_000_000.0;
                            println!("💳 SUI 余额: {:.9} SUI ({} MIST)", sui_amount, balance);
                        }
                    }
                }
                if let Some(coin_type) = result.get("coinType") {
                    println!("🪙 代币类型: {}", coin_type);
                }
            } else if let Some(error) = response.get("error") {
                println!("❌ 余额查询错误: {}", error);
            }
        }
        Err(e) => println!("⚠️  余额查询失败: {}", e),
    }
    
    // 2. 查询所有余额（包括其他代币）
    println!("\n💎 查询所有代币余额...");
    let all_balance_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "suix_getAllBalances",
        "params": [target_address]
    });
    
    match client.post(rpc_url)
        .json(&all_balance_request)
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                if let Some(balances) = result.as_array() {
                    println!("📊 总共找到 {} 种代币:", balances.len());
                    for (i, balance) in balances.iter().enumerate() {
                        if let (Some(coin_type), Some(total_balance)) = 
                            (balance.get("coinType"), balance.get("totalBalance")) {
                            if let Some(balance_str) = total_balance.as_str() {
                                if let Ok(balance_num) = balance_str.parse::<u64>() {
                                    let formatted_balance = if coin_type.as_str() == Some("0x2::sui::SUI") {
                                        format!("{:.9} SUI", balance_num as f64 / 1_000_000_000.0)
                                    } else {
                                        format!("{} units", balance_num)
                                    };
                                    println!("   {}. {}: {}", i + 1, coin_type, formatted_balance);
                                }
                            }
                        }
                    }
                }
            } else if let Some(error) = response.get("error") {
                println!("❌ 所有余额查询错误: {}", error);
            }
        }
        Err(e) => println!("⚠️  所有余额查询失败: {}", e),
    }
    
    // 3. 查询交易历史
    println!("\n📝 查询最近交易历史...");
    let tx_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "suix_queryTransactionBlocks",
        "params": [{
            "filter": {
                "FromAddress": target_address
            },
            "options": {
                "showInput": true,
                "showEffects": true,
                "showEvents": true,
                "showObjectChanges": false,
                "showBalanceChanges": true
            }
        }, null, 5, false]
    });
    
    match client.post(rpc_url)
        .json(&tx_request)
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                if let Some(data) = result.get("data") {
                    if let Some(transactions) = data.as_array() {
                        if transactions.is_empty() {
                            println!("📭 没有找到发送的交易");
                        } else {
                            println!("🎯 找到 {} 笔发送的交易:", transactions.len());
                            
                            for (i, tx) in transactions.iter().enumerate() {
                                println!("\n📋 交易 #{}", i + 1);
                                
                                if let Some(digest) = tx.get("digest") {
                                    println!("   📄 交易摘要: {}", digest);
                                }
                                
                                if let Some(timestamp) = tx.get("timestampMs") {
                                    if let Some(ts) = timestamp.as_str() {
                                        if let Ok(ts_ms) = ts.parse::<u64>() {
                                            let datetime = chrono::DateTime::from_timestamp_millis(ts_ms as i64)
                                                .unwrap_or_else(|| chrono::Utc::now());
                                            println!("   🕰️  时间: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC"));
                                        }
                                    }
                                }
                                
                                if let Some(effects) = tx.get("effects") {
                                    if let Some(status) = effects.get("status") {
                                        if let Some(status_obj) = status.as_object() {
                                            if status_obj.contains_key("success") {
                                                println!("   🎯 状态: ✅ 成功");
                                            } else if status_obj.contains_key("failure") {
                                                println!("   🎯 状态: ❌ 失败");
                                                if let Some(error) = status_obj.get("failure") {
                                                    println!("   🔍 错误: {}", error);
                                                }
                                            }
                                        }
                                    }
                                    
                                    if let Some(gas_used) = effects.get("gasUsed") {
                                        if let Some(computation_cost) = gas_used.get("computationCost") {
                                            println!("   ⛽ Gas 消耗: {}", computation_cost);
                                        }
                                    }
                                }
                                
                                // 显示余额变化
                                if let Some(balance_changes) = tx.get("balanceChanges") {
                                    if let Some(changes) = balance_changes.as_array() {
                                        for change in changes {
                                            if let (Some(owner), Some(coin_type), Some(amount)) = (
                                                change.get("owner").and_then(|o| o.get("AddressOwner")),
                                                change.get("coinType"),
                                                change.get("amount")
                                            ) {
                                                if let Some(amount_str) = amount.as_str() {
                                                    if let Ok(amount_num) = amount_str.parse::<i64>() {
                                                        let sui_amount = amount_num as f64 / 1_000_000_000.0;
                                                        let sign = if amount_num > 0 { "+" } else { "" };
                                                        println!("   💰 余额变化: {}{:.9} SUI ({})", sign, sui_amount, owner);
                                                        println!("      🪙 代币: {}", coin_type);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    println!("📭 没有找到交易数据");
                }
            } else if let Some(error) = response.get("error") {
                println!("❌ 交易查询错误: {}", error);
            }
        }
        Err(e) => println!("⚠️  交易查询失败: {}", e),
    }
    
    // 4. 查询接收的交易
    println!("\n📥 查询接收的交易...");
    let rx_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "suix_queryTransactionBlocks",
        "params": [{
            "filter": {
                "ToAddress": target_address
            },
            "options": {
                "showBalanceChanges": true,
                "showEffects": true
            }
        }, null, 3, false]
    });
    
    match client.post(rpc_url)
        .json(&rx_request)
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                if let Some(data) = result.get("data") {
                    if let Some(transactions) = data.as_array() {
                        if transactions.is_empty() {
                            println!("📭 没有找到接收的交易");
                        } else {
                            println!("📨 找到 {} 笔接收的交易:", transactions.len());
                            
                            for (i, tx) in transactions.iter().enumerate() {
                                println!("\n📋 接收交易 #{}", i + 1);
                                
                                if let Some(digest) = tx.get("digest") {
                                    println!("   📄 交易摘要: {}", digest);
                                }
                                
                                // 显示接收到的余额变化
                                if let Some(balance_changes) = tx.get("balanceChanges") {
                                    if let Some(changes) = balance_changes.as_array() {
                                        for change in changes {
                                            if let (Some(owner), Some(amount)) = (
                                                change.get("owner").and_then(|o| o.get("AddressOwner")),
                                                change.get("amount")
                                            ) {
                                                if owner.as_str() == Some(target_address) {
                                                    if let Some(amount_str) = amount.as_str() {
                                                        if let Ok(amount_num) = amount_str.parse::<i64>() {
                                                            if amount_num > 0 {
                                                                let sui_amount = amount_num as f64 / 1_000_000_000.0;
                                                                println!("   💰 接收: +{:.9} SUI", sui_amount);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(error) = response.get("error") {
                println!("❌ 接收交易查询错误: {}", error);
            }
        }
        Err(e) => println!("⚠️  接收交易查询失败: {}", e),
    }
    
    println!("\n🎉 地址查询完成!");
    println!("💡 提示: 如果没有看到交易，可能是因为:");
    println!("   1. 地址确实没有交易历史");
    println!("   2. 交易比较老，需要查询更多历史");
    println!("   3. 需要查询其他类型的交易过滤器");
    
    Ok(())
}