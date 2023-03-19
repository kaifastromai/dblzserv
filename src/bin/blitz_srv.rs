//!A simple server for Deblitz, a shameless rip off of Dutch Blitz but as a video game.
use blitz::{proto, server};
use tonic::transport::Server;

use tracing::{debug, info};
use tracing_subscriber;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "0.0.0.0:50051".parse()?;
    HttpServer::new(|| App::new().service(hello))
        .bind(addr)?
        .run()
        .await?;
    
    Ok(())
}
