//!A simple server for "blitz!", a shameless rip off of Dutch Blitz but as a video game.

use blitz::proto;
use blitz::server::*;
use tracing::info;
use tracing_subscriber;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let server = Server::new();
    let grpc_server = blitz::proto::session_service_server::SessionServiceServer::new(server);
    let addr = "[::1]:50051";
    tracing::info!("Starting game server on port 50051");
    tonic::transport::Server::builder()
        .add_service(grpc_server)
        .serve(addr.parse().unwrap())
        .await?;
    Ok(())
}
