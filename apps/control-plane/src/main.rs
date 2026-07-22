//! Moqentra control-plane HTTP entrypoint.
//!
//! Composes configuration, dependency injection, middleware ordering,
//! migrations, and lifecycle. Route handlers, DTOs, and the router live in
//! `moqentra-http-api`.

use axum::serve;
use moqentra_application::{
    InMemoryAnnotationRegistry, InMemoryDatasetRegistry, InMemoryModelRegistry,
    InMemoryTrainingRegistry,
};
use moqentra_auth::InMemoryAuditLog;
use moqentra_auth::{
    Authorizer, CompositeTokenValidator, HmacValidator, JwkSetValidator, OidcConfig,
    ServiceAccountValidator,
};
use moqentra_http_api::control_plane::{app_router, spawn_outbox_dispatcher, AppState};
use moqentra_object_store::{
    s3::{S3Config, S3ObjectStore},
    InMemoryObjectStore, InMemoryUploadSessionStore, ObjectStorage,
};
use moqentra_storage::{InMemoryOutbox, PgAuditLog};
use moqentra_types::config::SecretString;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

fn build_state_from_env() -> AppState {
    // OIDC configuration takes precedence for browser tokens.
    let oidc =
        std::env::var("MOQENTRA_OIDC_ISSUER")
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|issuer| {
                std::env::var("MOQENTRA_OIDC_AUDIENCE").ok().filter(|s| !s.is_empty()).map(
                    |audience| {
                        let mut config = OidcConfig::new(issuer, audience);
                        if let Ok(uri) = std::env::var("MOQENTRA_OIDC_JWKS_URI") {
                            if !uri.is_empty() {
                                config = config.with_jwks_uri(uri);
                            }
                        }
                        JwkSetValidator::new(config)
                    },
                )
            });

    // HMAC is kept only for local integration tests; OIDC is the production path.
    let jwt_secret = std::env::var("MOQENTRA_JWT_SECRET").ok().filter(|s| !s.is_empty());
    let issuer = std::env::var("MOQENTRA_JWT_ISSUER")
        .unwrap_or_else(|_| "https://moqentra.local".to_string());
    let audience =
        std::env::var("MOQENTRA_JWT_AUDIENCE").unwrap_or_else(|_| "moqentra".to_string());

    let hmac = jwt_secret.map(|secret| HmacValidator::new(secret, issuer, audience));

    let mut service_map = HashMap::new();
    if let Ok(raw) = std::env::var("MOQENTRA_SERVICE_TOKENS") {
        for part in raw.split(',') {
            if let Some((token, name)) = part.split_once('=') {
                if !token.is_empty() && !name.is_empty() {
                    service_map.insert(token.to_string(), name.to_string());
                }
            }
        }
    }
    let service = if service_map.is_empty() {
        None
    } else {
        Some(ServiceAccountValidator::new(service_map))
    };

    let mut tokens = CompositeTokenValidator::new(hmac, service);
    if let Some(oidc) = oidc {
        tokens = tokens.with_oidc(oidc);
    }
    let require_auth = std::env::var("MOQENTRA_REQUIRE_AUTH")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(tokens.is_configured());

    let object_store: Arc<dyn ObjectStorage + Send + Sync> =
        match std::env::var("MOQENTRA_S3_ENDPOINT") {
            Ok(endpoint) if !endpoint.is_empty() => {
                let config = S3Config {
                    bucket: std::env::var("MOQENTRA_S3_BUCKET").unwrap_or_default(),
                    endpoint,
                    region: std::env::var("MOQENTRA_S3_REGION")
                        .unwrap_or_else(|_| "us-east-1".to_string()),
                    access_key_id: SecretString::new(
                        std::env::var("MOQENTRA_S3_ACCESS_KEY_ID").unwrap_or_default(),
                    ),
                    secret_access_key: SecretString::new(
                        std::env::var("MOQENTRA_S3_SECRET_ACCESS_KEY").unwrap_or_default(),
                    ),
                    force_path_style: std::env::var("MOQENTRA_S3_FORCE_PATH_STYLE")
                        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                        .unwrap_or(true),
                };
                Arc::new(
                    S3ObjectStore::new(config)
                        .expect("S3 object store configuration must be valid"),
                )
            }
            _ => Arc::new(InMemoryObjectStore::new()),
        };

    let (audit, db_pool): (
        Arc<dyn moqentra_auth::AuditLog + Send + Sync>,
        Option<sqlx::PgPool>,
    ) = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => {
            let pool = sqlx::PgPool::connect_lazy(&url)
                .expect("DATABASE_URL must be a valid PostgreSQL connection string");
            (Arc::new(PgAuditLog::new(pool.clone())), Some(pool))
        }
        _ => (Arc::new(InMemoryAuditLog::new()), None),
    };

    AppState {
        ready: Arc::new(AtomicBool::new(true)),
        compiler: moqentra_application::ApplicationCompiler::new(),
        tokens,
        require_auth,
        authorizer: Arc::new(Mutex::new(Authorizer::new())),
        rate_limiters: Arc::new(Mutex::new(HashMap::new())),
        datasets: Arc::new(Mutex::new(InMemoryDatasetRegistry::new())),
        training: Arc::new(Mutex::new(InMemoryTrainingRegistry::new())),
        models: Arc::new(Mutex::new(InMemoryModelRegistry::new())),
        annotations: Arc::new(Mutex::new(InMemoryAnnotationRegistry::new())),
        outbox: Arc::new(InMemoryOutbox::new()),
        audit,
        object_store,
        upload_sessions: Arc::new(InMemoryUploadSessionStore::new()),
        upload_sig_secret: std::env::var("MOQENTRA_UPLOAD_SIG_SECRET")
            .unwrap_or_else(|_| "moqentra-upload-sig-secret".to_string()),
        db_pool,
        scheduler_url: std::env::var("MOQENTRA_SCHEDULER_URL").ok().filter(|s| !s.is_empty()),
        node_agent_url: std::env::var("MOQENTRA_NODE_AGENT_URL").ok().filter(|s| !s.is_empty()),
        http: reqwest::Client::new(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let listen =
        std::env::var("MOQENTRA_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = listen.parse()?;

    let state = build_state_from_env();

    if let Some(pool) = &state.db_pool {
        sqlx::migrate!("../../crates/storage/migrations")
            .run(pool)
            .await
            .map_err(|e| anyhow::anyhow!("database migration failed: {e}"))?;
    }

    tracing::info!(
        require_auth = state.require_auth,
        scheduler = ?state.scheduler_url,
        node_agent = ?state.node_agent_url,
        "moqentra-control-plane configured"
    );

    spawn_outbox_dispatcher(state.clone());
    let app = app_router(state);
    tracing::info!(%addr, "moqentra-control-plane listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve(listener, app).await?;
    Ok(())
}
