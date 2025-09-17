fn main() {
    // 注册Sui GraphQL schema
    let _schema_name = "SuiGQL";
    
    // 这是为了支持sui-graphql-client的schema生成
    println!("cargo:rerun-if-changed=build.rs");
}