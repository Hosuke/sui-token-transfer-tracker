use anyhow::Result;
use sui_token_transfer_tracker::sui_client::SuiClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 测试新的 SUI GraphQL 客户端");
    
    // 创建客户端
    let client = SuiClient::new("https://sui-mainnet.mystenlabs.com/graphql").await?;
    
    // 测试健康检查
    println!("📡 检查网络连接...");
    match client.health_check().await {
        Ok(true) => println!("✅ 网络连接正常"),
        Ok(false) => println!("❌ 网络连接失败"),
        Err(e) => println!("⚠️  网络检查错误: {}", e),
    }
    
    // 获取链ID
    println!("🔗 获取链ID...");
    match client.get_chain_id().await {
        Ok(chain_id) => println!("🆔 链ID: {}", chain_id),
        Err(e) => println!("❌ 获取链ID失败: {}", e),
    }
    
    println!("\n🎉 GraphQL客户端测试完成!");
    Ok(())
}