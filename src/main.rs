use rmcp::{ServiceExt, transport::stdio};
mod server;
use server::FundraiseServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[mcp-fundraise] v1.0.0");
    let service = FundraiseServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
