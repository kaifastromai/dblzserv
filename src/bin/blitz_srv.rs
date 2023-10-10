//!A simple server for "blitz!", a shameless rip off of Dutch Blitz but as a video game.



use blitz::server::*;
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let tracing_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(tracing_subscriber)?;
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
