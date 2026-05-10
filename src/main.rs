use cowboy::lobby::new_lobby_store;
use cowboy::server::{create_router, AppState};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        lobbies: new_lobby_store(),
    };

    let app = create_router(state);

    let addr = "0.0.0.0:3000";
    info!("Cowboy server starting on {addr}");
    println!("Cowboy server running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
