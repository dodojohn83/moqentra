//! Moqentra control-plane HTTP entrypoint.
//!
//! Composes configuration, dependency injection, middleware ordering,
//! migrations, and lifecycle. Route handlers, DTOs, and the router live in
//! `moqentra-http-api`.

use axum::serve;
mod artifact_validator;
use artifact_validator::AppArtifactValidator;
use moqentra_application::{
    InMemoryAnnotationRegistry, InMemoryConversionRegistry, InMemoryDatasetRegistry,
    InMemoryEvaluationRegistry, InMemoryModelRegistry, InMemoryTrainingRegistry,
};
use moqentra_auth::InMemoryAuditLog;
use moqentra_auth::{
    Authorizer, CompositeTokenValidator, HmacValidator, JwkSetValidator, OidcConfig,
    ServiceAccountValidator,
};
use moqentra_http_api::control_plane::{
    app_router, spawn_outbox_dispatcher, AppState, DatabaseHealthCheck, ObjectStorageHealthCheck,
};
use moqentra_object_store::{
    s3::{S3Config, S3ObjectStore},
    InMemoryObjectStore, InMemoryUploadSessionStore, ObjectStorage,
};
use moqentra_observability::{CompositeHealthCheck, LivenessCheck, MetricsRegistry};
use moqentra_storage::{InMemoryOutbox, PgAuditLog};
use moqentra_types::config::SecretString;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

fn parse_bool_env(var: &str) -> anyhow::Result<bool> {
    match std::env::var(var) {
        Ok(v) if v.eq_ignore_ascii_case("true") || v == "1" => Ok(true),
        Ok(v) if v.eq_ignore_ascii_case("false") || v == "0" || v.is_empty() => Ok(false),
        Ok(v) => Err(anyhow::anyhow!(
            "{var} must be 'true', 'false', '1', '0', or unset; got '{v}'"
        )),
        Err(_) => Ok(false),
    }
}

fn build_state_from_env() -> anyhow::Result<AppState> {
    // OIDC configuration takes precedence for browser tokens.
    let oidc = std::env::var("MOQENTRA_OIDC_ISSUER")
        .ok()
        .filter(|s| !s.is_empty())
        .zip(std::env::var("MOQENTRA_OIDC_AUDIENCE").ok().filter(|s| !s.is_empty()))
        .map(|(issuer, audience)| {
            let mut config = OidcConfig::new(issuer, audience);
            if let Ok(uri) = std::env::var("MOQENTRA_OIDC_JWKS_URI") {
                if !uri.is_empty() {
                    config = config.with_jwks_uri(uri);
                }
            }
            JwkSetValidator::new(config)
        })
        .transpose()?;

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
    let require_auth = match std::env::var("MOQENTRA_REQUIRE_AUTH") {
        Ok(v) if v.eq_ignore_ascii_case("true") || v == "1" => true,
        Ok(v) if v.eq_ignore_ascii_case("false") || v == "0" || v.is_empty() => false,
        Ok(v) => {
            return Err(anyhow::anyhow!(
                "MOQENTRA_REQUIRE_AUTH must be 'true', 'false', '1', '0', or unset; got '{v}'"
            ))
        }
        Err(_) => tokens.is_configured(),
    };

    let object_store: Arc<dyn ObjectStorage + Send + Sync> =
        match std::env::var("MOQENTRA_S3_ENDPOINT") {
            Ok(endpoint) if !endpoint.is_empty() => {
                let bucket = std::env::var("MOQENTRA_S3_BUCKET").unwrap_or_default();
                if bucket.is_empty() {
                    return Err(anyhow::anyhow!(
                        "MOQENTRA_S3_BUCKET is required when MOQENTRA_S3_ENDPOINT is set"
                    ));
                }
                let config = S3Config {
                    bucket,
                    endpoint,
                    region: std::env::var("MOQENTRA_S3_REGION")
                        .unwrap_or_else(|_| "us-east-1".to_string()),
                    access_key_id: SecretString::new(
                        std::env::var("MOQENTRA_S3_ACCESS_KEY_ID").unwrap_or_default(),
                    ),
                    secret_access_key: SecretString::new(
                        std::env::var("MOQENTRA_S3_SECRET_ACCESS_KEY").unwrap_or_default(),
                    ),
                    force_path_style: parse_bool_env("MOQENTRA_S3_FORCE_PATH_STYLE")?,
                };
                Arc::new(S3ObjectStore::new(config).map_err(|e| {
                    anyhow::anyhow!("S3 object store configuration is invalid: {e}")
                })?)
            }
            _ => Arc::new(InMemoryObjectStore::new()),
        };

    let (audit, db_pool): (
        Arc<dyn moqentra_auth::AuditLog + Send + Sync>,
        Option<sqlx::PgPool>,
    ) = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => {
            let pool = sqlx::PgPool::connect_lazy(&url)
                .map_err(|e| anyhow::anyhow!("DATABASE_URL is invalid: {e}"))?;
            (Arc::new(PgAuditLog::new(pool.clone())), Some(pool))
        }
        _ => (Arc::new(InMemoryAuditLog::new()), None),
    };

    let metrics = Arc::new(MetricsRegistry::default());
    let mut health = CompositeHealthCheck::new();
    health.add(Arc::new(LivenessCheck));
    health.add(Arc::new(ObjectStorageHealthCheck {
        store: object_store.clone(),
    }));
    if let Some(pool) = db_pool.clone() {
        health.add(Arc::new(DatabaseHealthCheck { pool }));
    }
    let health = Arc::new(health);

    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build http client: {e}"))?;

    Ok(AppState {
        compiler: moqentra_application::ApplicationCompiler::new(),
        tokens,
        require_auth,
        authorizer: Arc::new(Mutex::new(Authorizer::new())),
        rate_limiters: Arc::new(Mutex::new(HashMap::new())),
        metrics,
        health,
        datasets: Arc::new(Mutex::new(InMemoryDatasetRegistry::new())),
        training: Arc::new(Mutex::new(InMemoryTrainingRegistry::new())),
        models: Arc::new(Mutex::new(InMemoryModelRegistry::new())),
        conversions: Arc::new(Mutex::new(InMemoryConversionRegistry::new())),
        evaluations: Arc::new(Mutex::new(InMemoryEvaluationRegistry::new())),
        annotations: Arc::new(Mutex::new(InMemoryAnnotationRegistry::new())),
        outbox: Arc::new(InMemoryOutbox::new()),
        audit,
        object_store,
        upload_sessions: Arc::new(InMemoryUploadSessionStore::new()),
        upload_sig_secret: std::env::var("MOQENTRA_UPLOAD_SIG_SECRET")
            .unwrap_or_else(|_| "moqentra-upload-sig-secret".to_string()),
        import_jobs: Arc::new(moqentra_http_api::InMemoryImportJobStore::new()),
        media_validator: Arc::new(moqentra_object_store::DefaultMediaValidator),
        db_pool,
        scheduler_url: std::env::var("MOQENTRA_SCHEDULER_URL").ok().filter(|s| !s.is_empty()),
        node_agent_url: std::env::var("MOQENTRA_NODE_AGENT_URL").ok().filter(|s| !s.is_empty()),
        http,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let listen =
        std::env::var("MOQENTRA_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = listen.parse()?;

    let state = build_state_from_env()?;

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
    moqentra_http_api::import::spawn_import_worker(state.clone());
    moqentra_http_api::validation_worker::spawn_media_validation_worker(state.clone());
    moqentra_http_api::gc_worker::spawn_gc_worker(state.clone());

    let validator = Arc::new(AppArtifactValidator {
        training: state.training.clone(),
        models: state.models.clone(),
    });
    let worker_manager = moqentra_worker_control::SessionManager::new_with_validator(validator);
    let worker_service = moqentra_worker_control::WorkerControlService::new(worker_manager);
    let worker_listen =
        std::env::var("MOQENTRA_WORKER_GRPC_ADDR").unwrap_or_else(|_| "0.0.0.0:9090".to_string());
    let worker_addr: SocketAddr = worker_listen.parse()?;

    tokio::spawn(async move {
        let server = tonic::transport::Server::builder().add_service(
            moqentra_worker_control::worker_service_server(worker_service),
        );
        tracing::info!(%worker_addr, "moqentra worker gRPC listening");
        if let Err(e) = server.serve(worker_addr).await {
            tracing::error!(error = %e, "worker gRPC server failed");
        }
    });

    let app = app_router(state);
    tracing::info!(%addr, "moqentra-control-plane listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve(listener, app).await?;
    Ok(())
}
