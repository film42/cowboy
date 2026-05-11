use cowboy::lobby::new_lobby_store;
use cowboy::server::{AppState, create_router};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // LiveKit dev server defaults (livekit-server --dev)
    let livekit_api_key = std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "devkey".to_string());
    let livekit_api_secret =
        std::env::var("LIVEKIT_API_SECRET").unwrap_or_else(|_| "secret".to_string());
    let livekit_url =
        std::env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://localhost:7880".to_string());

    let state = AppState {
        lobbies: new_lobby_store(),
        livekit_api_key,
        livekit_api_secret,
        livekit_url,
    };

    let app = create_router(state);

    let addr = "0.0.0.0:3000";
    info!("Cowboy server starting on {addr}");
    println!("Cowboy server running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
