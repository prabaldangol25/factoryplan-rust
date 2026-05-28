use actix_cors::Cors;
use actix_web::{get, web, App, HttpServer, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[get("/api/health")]
async fn health() -> impl Responder {
    web::Json(Health {
        status: "ok",
        service: "factoryplan-backend",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    log::info!("factoryplan-backend starting on {host}:{port}");

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new().wrap(cors).service(health)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
