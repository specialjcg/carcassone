use std::net::SocketAddr;

use carcassonne_backend::api::{new_store, router};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let store = new_store();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = router(store).layer(cors);
    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    tracing::info!("carcassonne-backend listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
