mod interfaces;
mod routes;
mod state;

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::state::AppState;
use crate::routes::create_router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialisation du logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialisation de l'état global
    let state = AppState::new();

    // Création du router
    let app = create_router(state);

    // Lancement du serveur
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Pylos Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
