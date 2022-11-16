//!A simple server for Deblitz, a shameless rip off of Dutch Blitz but as a video game.

mod db;
pub mod blitz;
mod server;
mod proto;


#[tokio::main]
async fn main() {
    let _client=proto::game_service_client::GameServiceClient::connect("http://[::1]:50051").await.unwrap();
}
