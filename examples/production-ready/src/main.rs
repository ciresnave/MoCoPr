mod production_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    production_server::run_server().await
}
