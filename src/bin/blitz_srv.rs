//!A simple server for "blitz!", a shameless rip off of Dutch Blitz but as a video game.

use std::sync::Arc;

use blitz::proto;
use blitz::server::*;
use tracing::info;
use tracing_subscriber;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let server = Server::new();
    let session_server =
        blitz::proto::session_service_server::SessionServiceServer::new(server.clone());
    let game_server = blitz::proto::game_service_server::GameServiceServer::new(server);
    let addr = "[::1]:50051";
    tracing::info!("Starting game server on port 50051");
    tonic::transport::Server::builder()
        .add_service(session_server)
        .add_service(game_server)
        .serve(addr.parse().unwrap())
        .await?;
    Ok(())
}
