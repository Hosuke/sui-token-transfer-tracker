// build.rs
fn main() {
    let schema_name = "SUI_TRACKER_SCHEMA";
    sui_graphql_client_build::register_schema(schema_name);
}