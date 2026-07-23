//! Moqentra dyun-agent: replica lifecycle + health.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use moqentra_dyun_adapter::dyun::{
    AgentCapabilities, DyunGraphBundle, Replica, ReplicaState, ResourceLimits,
};
use moqentra_types::{
    DeploymentId, NodeId, ProjectId, RandomIdGenerator, ReplicaId, TenantId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AgentState {
    capabilities: Arc<AgentCapabilities>,
    replicas: Arc<Mutex<HashMap<ReplicaId, Replica>>>,
    sandbox_root: String,
    persistence_path: PathBuf,
    production_keys: Arc<Vec<String>>,
    dev_keys: Arc<Vec<String>>,
    object_store: Arc<dyn moqentra_object_store::ObjectStorage + Send + Sync>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    node_id: String,
    replica_count: usize,
    running: usize,
    sandbox_root: String,
}

#[derive(Deserialize)]
struct CreateReplicaRequest {
    deployment_id: String,
    generation: u64,
    fencing_token: u64,
    application_digest: String,
    graph_spec: moqentra_domain::application::GraphSpec,
    graph_spec_digest: String,
    #[serde(default)]
    artifact_bindings: BTreeMap<String, moqentra_dyun_adapter::dyun::ArtifactBinding>,
    runtime_profile: String,
    #[serde(default)]
    compatible_dg_versions: Vec<String>,
    signature: String,
}

#[derive(Deserialize)]
struct TransitionRequest {
    generation: u64,
    fencing_token: u64,
    state: String,
    #[serde(default)]
    observed_generation: u64,
}

async fn healthz(State(state): State<AgentState>) -> Json<HealthResponse> {
    let replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let running = replicas.values().filter(|r| matches!(r.state, ReplicaState::Running)).count();
    Json(HealthResponse {
        status: "ok",
        service: "moqentra-dyun-agent",
        node_id: state.capabilities.node_id.to_string(),
        replica_count: replicas.len(),
        running,
        sandbox_root: state.sandbox_root.clone(),
    })
}

async fn capabilities(State(state): State<AgentState>) -> Json<AgentCapabilities> {
    Json(state.capabilities.as_ref().clone())
}

async fn list_replicas(State(state): State<AgentState>) -> Json<Vec<serde_json::Value>> {
    let replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let list = replicas
        .values()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "deployment_id": r.deployment_id.to_string(),
                "generation": r.generation,
                "state": format!("{:?}", r.state),
                "desired_state": format!("{:?}", r.desired_state),
                "node_id": r.node_id.to_string(),
            })
        })
        .collect();
    Json(list)
}

async fn create_replica(
    State(state): State<AgentState>,
    Json(req): Json<CreateReplicaRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let gen = RandomIdGenerator;
    let deployment_id = DeploymentId::try_from(req.deployment_id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let bundle = DyunGraphBundle {
        version: "DyunGraphBundle/v1".to_string(),
        application_digest: req.application_digest,
        graph_spec: req.graph_spec,
        graph_spec_digest: req.graph_spec_digest,
        artifact_bindings: req.artifact_bindings,
        runtime_profile: req.runtime_profile,
        resource_limits: ResourceLimits {
            cpu_milli: 2000,
            memory_mib: 4096,
            gpu_count: 0,
        },
        signature: req.signature,
        compatible_dg_versions: if req.compatible_dg_versions.is_empty() {
            vec!["dg-1.0".to_string()]
        } else {
            req.compatible_dg_versions
        },
    };
    let mut replica = Replica::new(
        ReplicaId::new_v7(&gen),
        deployment_id,
        req.generation,
        req.fencing_token,
        bundle,
        state.capabilities.node_id,
    );
    replica.verify_bundle(&state.production_keys, &state.dev_keys).map_err(|e| {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    replica
        .transition(req.generation, req.fencing_token, ReplicaState::Preparing)
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    let id = replica.id;
    state.replicas.lock().unwrap_or_else(|e| e.into_inner()).insert(id, replica);
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id.to_string(),
            "state": "Preparing",
        })),
    ))
}

fn parse_state(s: &str) -> Option<ReplicaState> {
    match s {
        "Pending" => Some(ReplicaState::Pending),
        "Preparing" => Some(ReplicaState::Preparing),
        "Starting" => Some(ReplicaState::Starting),
        "Running" => Some(ReplicaState::Running),
        "Draining" => Some(ReplicaState::Draining),
        "Stopped" => Some(ReplicaState::Stopped),
        "Failed" => Some(ReplicaState::Failed),
        _ => None,
    }
}

async fn transition_replica(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let replica_id = ReplicaId::try_from(id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let target = parse_state(&req.state).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid state"})),
        )
    })?;
    let drain_timeout = std::env::var("MOQENTRA_DYUN_DRAIN_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let replica = replicas.get_mut(&replica_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "replica not found"})),
        )
    })?;
    if target == ReplicaState::Draining {
        replica.drain(std::time::Duration::from_secs(drain_timeout));
    } else {
        replica.transition(req.generation, req.fencing_token, target).map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    }
    Ok(Json(serde_json::json!({
        "id": replica.id.to_string(),
        "state": format!("{:?}", replica.state),
    })))
}

async fn heartbeat_replica(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let replica_id = ReplicaId::try_from(id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let replica = replicas.get_mut(&replica_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "replica not found"})),
        )
    })?;
    replica
        .heartbeat(req.generation, req.fencing_token, req.observed_generation)
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(serde_json::json!({
        "id": replica.id.to_string(),
        "last_heartbeat": replica.last_heartbeat.map(|t| t.to_string()),
    })))
}

#[derive(Deserialize)]
struct PrepareRequest {
    generation: u64,
    fencing_token: u64,
}

async fn prepare_replica(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(req): Json<PrepareRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let replica_id = ReplicaId::try_from(id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Extract bindings while holding the lock, then release it before any await.
    let (base_dir, bindings) = {
        let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
        let replica = replicas.get_mut(&replica_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "replica not found"})),
            )
        })?;

        if req.generation != replica.generation || req.fencing_token != replica.fencing_token {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "stale prepare command"})),
            ));
        }

        let base_dir =
            PathBuf::from(&state.sandbox_root).join(replica.id.to_string()).join("models");
        std::fs::create_dir_all(&base_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

        let bindings: Vec<_> = replica.bundle.artifact_bindings.clone().into_iter().collect();
        (base_dir, bindings)
    };

    let paths = materialize_artifacts(state.object_store.clone(), base_dir, bindings).await?;

    let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(replica) = replicas.get_mut(&replica_id) {
        replica.updated_at = moqentra_types::UtcTimestamp::now();
    }

    Ok(Json(serde_json::json!({
        "id": replica_id.to_string(),
        "prepared": paths,
    })))
}

async fn materialize_artifacts(
    object_store: Arc<dyn moqentra_object_store::ObjectStorage + Send + Sync>,
    base_dir: PathBuf,
    bindings: Vec<(String, moqentra_dyun_adapter::dyun::ArtifactBinding)>,
) -> Result<Vec<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut paths = Vec::new();
    for (slot, binding) in bindings {
        if !moqentra_types::valid_content_digest(&binding.digest) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid digest for slot {slot}")})),
            ));
        }
        let parts: Vec<&str> = binding.object_key.split('/').collect();
        if parts.len() < 4 || parts[0] != "tenants" || parts[2] != "projects" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({"error": format!("object key for slot {slot} is not tenant-scoped")}),
                ),
            ));
        }
        let tenant_id = TenantId::from_str(parts[1]).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid tenant in object key for slot {slot}: {e}")})),
            )
        })?;
        let project_id = ProjectId::from_str(parts[3]).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid project in object key for slot {slot}: {e}")})),
            )
        })?;
        let object_key = moqentra_object_store::ObjectKey::from_str(tenant_id, project_id, &binding.object_key)
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("invalid object key for slot {slot}: {e}")})),
                )
            })?;
        let _ = object_key;

        let (data, meta) = object_store.get_object(&binding.object_key).await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("failed to fetch {slot}: {e}")})),
            )
        })?;

        use sha2::{Digest, Sha256};
        let computed = format!("sha256:{:x}", Sha256::digest(&data));
        if computed != binding.digest {
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("digest mismatch for slot {slot}")})),
            ));
        }

        let digest = binding.digest.split_once(':').map(|(_, h)| h).unwrap_or(&binding.digest);
        let artifact_dir = base_dir.join(digest);
        std::fs::create_dir_all(&artifact_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
        let file_path = artifact_dir.join("artifact");
        tokio::fs::write(&file_path, data).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut dir_perms = std::fs::metadata(&artifact_dir).unwrap().permissions();
            dir_perms.set_mode(0o555);
            std::fs::set_permissions(&artifact_dir, dir_perms).ok();
            let mut file_perms = std::fs::metadata(&file_path).unwrap().permissions();
            file_perms.set_mode(0o444);
            std::fs::set_permissions(&file_path, file_perms).ok();
        }

        paths.push(serde_json::json!({
            "slot": slot,
            "object_key": binding.object_key,
            "path": file_path.to_string_lossy().to_string(),
            "size": meta.size,
            "media_type": binding.media_type,
        }));
    }

    Ok(paths)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct DyunManifest {
    version: String,
    #[serde(default)]
    commit: String,
    #[serde(default)]
    schemas: Vec<String>,
    #[serde(default)]
    elements: Vec<String>,
    #[serde(default)]
    codecs: Vec<String>,
    #[serde(default)]
    backends: Vec<String>,
    #[serde(default)]
    build_features: Vec<String>,
}

fn comma_list(env_var: &str) -> Vec<String> {
    std::env::var(env_var)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn discover_capabilities() -> AgentCapabilities {
    let gen = RandomIdGenerator;
    // Capabilities are probed from a manifest pinned to a fixed commit at build
    // time. Runtime overrides are allowed for accelerators/version, but the
    // element schemas and backends come from the committed manifest.
    let manifest: DyunManifest = serde_json::from_str(
        option_env!("MOQENTRA_DYUN_MANIFEST_JSON")
            .unwrap_or_else(|| include_str!("../dyun-manifest.json")),
    )
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "failed to parse dyun manifest, using empty defaults");
        DyunManifest {
            version: "dg/v1".to_string(),
            commit: String::new(),
            schemas: Vec::new(),
            elements: Vec::new(),
            codecs: Vec::new(),
            backends: Vec::new(),
            build_features: Vec::new(),
        }
    });

    let accelerators = if std::env::var("MOQENTRA_DG_ACCELERATORS").is_ok() {
        comma_list("MOQENTRA_DG_ACCELERATORS")
    } else {
        // Default to CPU only; real GPU discovery is a follow-up probe.
        vec!["cpu".to_string()]
    };

    AgentCapabilities {
        node_id: NodeId::new_v7(&gen),
        agent_version: std::env::var("MOQENTRA_DYUN_AGENT_VERSION")
            .unwrap_or_else(|_| "0.1.0".to_string()),
        dg_versions: vec![manifest.version],
        codecs: if std::env::var("MOQENTRA_DG_CODECS").is_ok() {
            comma_list("MOQENTRA_DG_CODECS")
        } else {
            manifest.codecs
        },
        accelerators,
        element_schemas: manifest.schemas,
        backends: manifest.backends,
        build_features: manifest.build_features,
        max_replicas: std::env::var("MOQENTRA_DYUN_MAX_REPLICAS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8),
    }
}

fn build_object_store_from_env(
) -> anyhow::Result<Arc<dyn moqentra_object_store::ObjectStorage + Send + Sync>> {
    use moqentra_object_store::{InMemoryObjectStore, ObjectStorage, S3ObjectStore};
    if let Ok(bucket) = std::env::var("MOQENTRA_S3_BUCKET") {
        let endpoint = std::env::var("MOQENTRA_S3_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".to_string());
        let region =
            std::env::var("MOQENTRA_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let access_key_id = std::env::var("MOQENTRA_S3_ACCESS_KEY_ID").unwrap_or_default();
        let secret_access_key = std::env::var("MOQENTRA_S3_SECRET_ACCESS_KEY").unwrap_or_default();
        let force_path_style =
            std::env::var("MOQENTRA_S3_FORCE_PATH_STYLE").ok().as_deref() != Some("false");
        let config = moqentra_object_store::s3::S3Config {
            bucket,
            endpoint,
            region,
            access_key_id: moqentra_types::config::SecretString::new(access_key_id),
            secret_access_key: moqentra_types::config::SecretString::new(secret_access_key),
            force_path_style,
        };
        let store = S3ObjectStore::new(config).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(Arc::new(store) as Arc<dyn ObjectStorage + Send + Sync>)
    } else {
        Ok(Arc::new(InMemoryObjectStore::new()) as Arc<dyn ObjectStorage + Send + Sync>)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let caps = discover_capabilities();
    let sandbox_root =
        std::env::var("MOQENTRA_DYUN_SANDBOX").unwrap_or_else(|_| "/tmp/moqentra-dyun".to_string());
    std::fs::create_dir_all(&sandbox_root)?;
    let production_keys = std::env::var("MOQENTRA_DYUN_PRODUCTION_KEYS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let dev_keys = std::env::var("MOQENTRA_DYUN_DEV_KEYS")
        .unwrap_or_else(|_| "trusted:".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let object_store = build_object_store_from_env()?;
    let persistence_path = PathBuf::from(&sandbox_root).join("replicas.json");
    let initial_replicas = load_replicas(&persistence_path).await.unwrap_or_default();
    let state = AgentState {
        capabilities: Arc::new(caps),
        replicas: Arc::new(Mutex::new(initial_replicas)),
        sandbox_root,
        persistence_path,
        production_keys: Arc::new(production_keys),
        dev_keys: Arc::new(dev_keys),
        object_store,
    };

    let listen =
        std::env::var("MOQENTRA_DYUN_AGENT_ADDR").unwrap_or_else(|_| "0.0.0.0:8083".to_string());
    let addr: SocketAddr = listen.parse()?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/replicas", get(list_replicas).post(create_replica))
        .route("/v1/replicas/{id}/prepare", post(prepare_replica))
        .route("/v1/replicas/{id}/transition", post(transition_replica))
        .route("/v1/replicas/{id}/heartbeat", post(heartbeat_replica))
        .with_state(state.clone());

    let heartbeat_timeout = std::env::var("MOQENTRA_DYUN_HEARTBEAT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    tokio::spawn(reconcile_replicas(
        state,
        std::time::Duration::from_secs(heartbeat_timeout),
    ));

    tracing::info!(%addr, "moqentra-dyun-agent listening");
    let _ = UtcTimestamp::now();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn reconcile_replicas(state: AgentState, heartbeat_timeout: std::time::Duration) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        {
            let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
            let now = UtcTimestamp::now();
            for replica in replicas.values_mut() {
                replica.reconcile(now, heartbeat_timeout);
            }
        }
        if let Err(e) = save_replicas(&state).await {
            tracing::warn!(error = %e, "failed to persist replica state");
        }
    }
}

async fn load_replicas(path: &PathBuf) -> anyhow::Result<HashMap<ReplicaId, Replica>> {
    let bytes = tokio::fs::read(path).await?;
    let replicas: HashMap<ReplicaId, Replica> = serde_json::from_slice(&bytes)?;
    Ok(replicas)
}

async fn save_replicas(state: &AgentState) -> anyhow::Result<()> {
    let json = {
        let replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
        serde_json::to_vec_pretty(&*replicas)?
    };
    tokio::fs::write(&state.persistence_path, json).await?;
    Ok(())
}
