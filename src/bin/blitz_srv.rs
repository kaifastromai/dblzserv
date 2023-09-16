//!A simple server for "blitz!", a shameless rip off of Dutch Blitz but as a video game.

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use blitz::server::*;
use tracing::info;
use tracing_subscriber;
#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let data = web::Data::new(blitz::server::Server::new());
    let addr = "0.0.0.0:50055";
    info!("Starting server on {}", addr);
    HttpServer::new(move || {
        App::new().app_data(data.clone()).service(
            web::scope("/api")
                .service(create_session)
                .service(join_session)
                .service(show_sessions),
        )
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
