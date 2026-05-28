use std::path::PathBuf;

use pylos_server::routes::create_router;
use pylos_server::state::AppState;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialisation du logging structuré (JSON ou texte)
    let env_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    );

    if std::env::var("LOG_FORMAT")
        .map(|s| s.to_lowercase())
        .unwrap_or_default()
        == "json"
    {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Pylos AI Gateway"
    );

    // Chemin du fichier de config (priorité : PYLOS_CONFIG env var, puis pylos.json local)
    let config_path = std::env::var("PYLOS_CONFIG")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let p = PathBuf::from("pylos.json");
            if p.exists() {
                Some(p)
            } else {
                None
            }
        });

    if let Some(ref p) = config_path {
        tracing::info!(path = %p.display(), "Using config file");
    }

    // Construction de l'état depuis la config
    let state = AppState::from_config(config_path).await?;

    // Warning si l'API management n'est pas protégée
    if state.admin_key.is_none() {
        tracing::warn!(
            "PYLOS_ADMIN_KEY is not set — management API (/providers, /virtual-keys, /config) is unprotected"
        );
    } else {
        tracing::info!("Management API protected with PYLOS_ADMIN_KEY");
    }

    // Port depuis la config ou PORT env var (env var prioritaire pour docker/k8s)
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(state.config_store.get_port().await);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    // Création du router Axum
    let app = create_router(state);

    tracing::info!("Pylos listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
