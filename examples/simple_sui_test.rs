use anyhow::Result;
use sui_graphql_client::Client;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let target_address = "0xaf63b1dbc01a2504d42606e3c57bca22c32c3ef86e809e7694a9fbfdac714dee";
    
    println!("🔍 正在查询 SUI 地址: {}", target_address);
    println!("================================================");
    
    // 连接到主网
    let client = Client::new_mainnet();
    
    // 测试网络连接
    match client.chain_id().await {
        Ok(chain_id) => {
            println!("✅ 网络连接成功");
            println!("🌐 链 ID: {}", chain_id);
        }
        Err(e) => {
            println!("❌ 网络连接失败: {}", e);
            return Err(e.into());
        }
    }
    
    println!("\n🎯 基本连接测试成功!");
    println!("📝 注意: 完整的地址查询功能需要正确的 GraphQL schema 定义");
    println!("💡 可以使用以下方式继续开发:");
    println!("   1. 使用 sui-sdk-types 直接调用 JSON-RPC");
    println!("   2. 实现自定义的 GraphQL 查询类型");
    println!("   3. 使用 reqwest 直接调用 RPC 接口");
    
    // 显示地址信息
    println!("\n📍 目标地址详情:");
    println!("   地址: {}", target_address);
    println!("   长度: {} 字符", target_address.len());
    println!("   格式: {}", if target_address.starts_with("0x") && target_address.len() == 66 { "✅ 有效" } else { "❌ 无效" });
    
    Ok(())
}