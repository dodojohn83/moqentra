//! Control-plane route handlers, DTOs, shared application state, and router.
//!
//! The application binary in `apps/control-plane` is responsible for
//! configuration, dependency injection, middleware ordering, and lifecycle.
//!
//! Health endpoints stay unauthenticated.

use crate::import::ImportJobStore;
use crate::northbound::{ProblemDetails, TokenBucketLimiter};
use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, MatchedPath, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{middleware, Json, Router};
use bytes::Bytes;
use moqentra_application::{
    plan_dispatch, ApplicationCompiler, ArtifactReconciler, CompileResult, DispatchAction,
    InMemoryAnnotationRegistry, InMemoryConversionRegistry, InMemoryDatasetRegistry,
    InMemoryEvaluationRegistry, InMemoryModelRegistry, InMemoryTrainingRegistry,
};
use moqentra_auth::{
    Action, AuditCategory, AuditEvent, AuditLog, AuditOutcome, Authorizer, CompositeTokenValidator,
    Resource, Role, RoleStore, Scope,
};
use moqentra_domain::annotation::{AnnotationProject, Ontology, TaskType};
use moqentra_domain::application::{ApplicationSpec, ArtifactBinding};
use moqentra_domain::conversion::{ConversionJob, ConversionProfile, EvaluationRun};
use moqentra_domain::dataset::{AssetRef, Dataset, DatasetVersion};
use moqentra_domain::import::ImportJob;
use moqentra_domain::model_registry::{Model, ModelVersion};
use moqentra_domain::training::{
    DistributedConfig, Experiment, ParameterSchema, ResourceRequest, TrainingJob, TrainingJobSpec,
};
use moqentra_object_store::{
    upload_session::part_digest, ObjectKey, ObjectStorage, UploadSession, UploadSessionStore,
};
use moqentra_observability::{CompositeHealthCheck, HealthCheck, HealthReport, MetricsRegistry};
use moqentra_storage::{InMemoryOutbox, OutboxEvent, OutboxStatus, OutboxStore, PgRoleStore};
use moqentra_types::{
    AnnotationProjectId, AssetId, ConversionJobId, DatasetId, DatasetVersionId, Error,
    EvaluationRunId, ExperimentId, ModelId, ModelVersionId, Page, PageRequest, Principal,
    ProjectId, RandomIdGenerator, RequestContext, TenantId, TrainingJobId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use tracing::Instrument;
use uuid::Uuid;

/// Application state shared by all control-plane handlers.
#[derive(Clone)]
pub struct AppState {
    pub compiler: ApplicationCompiler,
    pub tokens: CompositeTokenValidator,
    /// When true, protected routes require a valid bearer token.
    pub require_auth: bool,
    pub authorizer: Arc<Mutex<Authorizer>>,
    pub rate_limiters: Arc<Mutex<HashMap<TenantId, TokenBucketLimiter>>>,
    pub metrics: Arc<MetricsRegistry>,
    pub health: Arc<CompositeHealthCheck>,
    pub datasets: Arc<Mutex<InMemoryDatasetRegistry>>,
    pub training: Arc<Mutex<InMemoryTrainingRegistry>>,
    pub models: Arc<Mutex<InMemoryModelRegistry>>,
    pub conversions: Arc<Mutex<InMemoryConversionRegistry>>,
    pub evaluations: Arc<Mutex<InMemoryEvaluationRegistry>>,
    pub annotations: Arc<Mutex<InMemoryAnnotationRegistry>>,
    pub outbox: Arc<InMemoryOutbox>,
    pub audit: Arc<dyn AuditLog + Send + Sync>,
    /// Object storage backend for artifacts.
    pub object_store: Arc<dyn ObjectStorage + Send + Sync>,
    /// Upload session state.
    pub upload_sessions: Arc<dyn UploadSessionStore + Send + Sync>,
    /// HMAC secret used to sign part upload URLs.
    pub upload_sig_secret: String,
    /// Import job store.
    pub import_jobs: Arc<dyn ImportJobStore + Send + Sync>,
    /// Media validator used before publishing a dataset version.
    pub media_validator: Arc<dyn moqentra_object_store::MediaValidator + Send + Sync>,
    /// Optional PostgreSQL pool for audit / repository persistence.
    pub db_pool: Option<sqlx::PgPool>,
    /// Optional downstream URLs for outbox side-effects.
    pub scheduler_url: Option<String>,
    pub node_agent_url: Option<String>,
    pub http: reqwest::Client,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Deserialize)]
struct CompileRequest {
    spec: ApplicationSpec,
}

#[derive(Deserialize)]
struct CreateDatasetRequest {
    name: String,
    project_id: String,
}

#[derive(Serialize, Deserialize)]
struct DatasetResponse {
    id: String,
    tenant_id: String,
    project_id: String,
    name: String,
    state: String,
}

#[derive(Serialize)]
struct WhoAmIResponse {
    principal: Principal,
    tenant_id: String,
    request_id: String,
}

pub(crate) struct ApiError {
    status: StatusCode,
    problem: ProblemDetails,
}

impl From<Error> for ApiError {
    fn from(err: Error) -> Self {
        let status = StatusCode::from_u16(err.kind.http_status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            problem: ProblemDetails::from_error(&err, "control-plane"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.problem)).into_response()
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer "))?;
    Some(token.trim().to_string())
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

pub(crate) async fn resolve_context(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<RequestContext, Error> {
    let request_id = header_str(headers, "x-request-id")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("req-{}", UtcTimestamp::now()));

    let tenant_raw = header_str(headers, "x-tenant-id")
        .ok_or_else(|| Error::invalid_argument("X-Tenant-Id header is required"))?;
    let tenant_id = TenantId::try_from(tenant_raw.as_str())?;

    let project_header =
        header_str(headers, "x-project-id").and_then(|p| ProjectId::try_from(p.as_str()).ok());

    let principal = if let Some(token) = bearer_token(headers) {
        let session = state.tokens.validate_session(&token).await?;
        if let Principal::User { id } = session.principal {
            // Resolve roles from DB membership when available; JWT claim roles are ignored in
            // production to enforce re-auth on tenant/project switch.
            let roles = if let Some(pool) = &state.db_pool {
                let store = PgRoleStore::new(pool.clone());
                match project_header {
                    Some(pid) => store.project_roles(id, tenant_id, pid).await?,
                    None => store.tenant_roles(id, tenant_id).await?,
                }
            } else {
                session.roles
            };
            let mut authz = state.authorizer.lock().unwrap_or_else(|e| e.into_inner());
            for role in roles {
                authz.assign_role(id, tenant_id, role);
            }
            // Optional project membership bootstrap for development.
            if let Some(project_id) = project_header {
                authz.add_project_member(id, tenant_id, project_id);
            }
        }
        if let Principal::Service { name } = &session.principal {
            let mut authz = state.authorizer.lock().unwrap_or_else(|e| e.into_inner());
            // Default operator grant for known services unless already assigned.
            authz.assign_service_role(name.clone(), tenant_id, Role::Operator);
        }
        session.principal
    } else if state.require_auth {
        return Err(Error::unauthenticated(
            "Authorization bearer token required",
        ));
    } else {
        // Dev open mode: anonymous principal for local tooling.
        Principal::Anonymous
    };

    let mut ctx = RequestContext::new(tenant_id, principal, request_id);
    if let Some(project_id) = project_header {
        ctx = ctx.with_project(project_id);
    }
    Ok(ctx)
}

async fn emit_event(
    state: &AppState,
    tenant_id: TenantId,
    aggregate_type: &str,
    aggregate_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) {
    let event = OutboxEvent {
        id: Uuid::new_v4(),
        tenant_id,
        aggregate_type: aggregate_type.to_string(),
        aggregate_id: aggregate_id.to_string(),
        event_type: event_type.to_string(),
        payload,
        status: OutboxStatus::Pending,
        retry_count: 0,
        failure_reason: None,
        created_at: UtcTimestamp::now(),
    };
    let _ = state.outbox.append(event).await;
}

pub(crate) fn resolve_project_id(
    ctx: &RequestContext,
    body_project_id: &str,
) -> Result<ProjectId, Error> {
    if let Some(p) = ctx.project_id {
        if !body_project_id.is_empty() {
            let body = ProjectId::try_from(body_project_id)?;
            if body != p {
                return Err(Error::invalid_argument(
                    "X-Project-Id does not match body project_id",
                ));
            }
        }
        Ok(p)
    } else {
        ProjectId::try_from(body_project_id)
    }
}

pub(crate) fn check_rate_limit(state: &AppState, tenant_id: TenantId) -> Result<(), Error> {
    let now = UtcTimestamp::now();
    let mut map = state.rate_limiters.lock().unwrap_or_else(|e| e.into_inner());
    let limiter = map.entry(tenant_id).or_insert_with(|| {
        TokenBucketLimiter::new(tenant_id, 100, 50.0, now)
            .expect("static rate limit config is valid")
    });
    let window = limiter.try_acquire(now, 1)?;
    if window.limited {
        return Err(Error::rate_limited("tenant request rate exceeded"));
    }
    Ok(())
}

pub(crate) async fn authorize(
    state: &AppState,
    ctx: &RequestContext,
    action: Action,
    resource: Resource,
) -> Result<(), Error> {
    // Open-dev anonymous: allow only when auth is not required.
    if matches!(ctx.principal, Principal::Anonymous) && !state.require_auth {
        return Ok(());
    }
    let mut scope = Scope::new(ctx.tenant_id);
    if let Some(project_id) = ctx.project_id {
        scope = scope.with_project(project_id);
    }
    let outcome = {
        let authz = state.authorizer.lock().unwrap_or_else(|e| e.into_inner());
        match authz.authorize(ctx, action, resource, &scope) {
            Ok(_) => AuditOutcome::Success,
            Err(_) => AuditOutcome::Denied,
        }
    };
    let event = AuditEvent {
        event_id: format!("evt-{}", Uuid::new_v4()),
        category: AuditCategory::Authorization,
        actor: ctx.principal.clone(),
        tenant_id: ctx.tenant_id,
        project_id: ctx.project_id,
        action: format!("{action:?}"),
        resource: format!("{resource:?}"),
        outcome,
        reason: None,
        correlation_id: ctx.request_id.clone(),
        occurred_at: UtcTimestamp::now(),
    };
    if let Err(e) = state.audit.record(event).await {
        tracing::warn!("failed to record authorization audit event: {}", e);
    }
    match outcome {
        AuditOutcome::Success => Ok(()),
        _ => Err(Error::permission_denied("not authorized for this action")),
    }
}

pub(crate) async fn audit_write(
    state: &AppState,
    ctx: &RequestContext,
    category: AuditCategory,
    action: &str,
    resource_type: Resource,
    resource_id: Option<&str>,
    outcome: AuditOutcome,
) {
    let mut resource = format!("{resource_type:?}");
    if let Some(id) = resource_id {
        resource.push(':');
        resource.push_str(id);
    }
    let event = AuditEvent {
        event_id: format!("evt-{}", Uuid::new_v4()),
        category,
        actor: ctx.principal.clone(),
        tenant_id: ctx.tenant_id,
        project_id: ctx.project_id,
        action: action.to_string(),
        resource,
        outcome,
        reason: None,
        correlation_id: ctx.request_id.clone(),
        occurred_at: UtcTimestamp::now(),
    };
    if let Err(e) = state.audit.record(event).await {
        tracing::warn!("failed to record write audit event: {}", e);
    }
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "moqentra-control-plane",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let report = state.health.ready().await;
    let status = if report.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

async fn livez() -> impl IntoResponse {
    Json(HealthReport {
        status: "ok",
        ready: true,
        checks: vec![],
    })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let body = state.metrics.prometheus();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}

/// Middleware that records request latency and counter metrics and propagates
/// trace/correlation/request IDs.  Resource IDs are never used as metric labels.
async fn observability_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let matched_path = req.extensions().get::<MatchedPath>().map(|p| p.as_str().to_string());
    let route = matched_path.unwrap_or_else(|| "unknown".to_string());
    let request_id = header_str(req.headers(), "x-request-id")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("req-{}", UtcTimestamp::now()));
    let correlation_id = header_str(req.headers(), "x-correlation-id");
    let trace_id = req
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_traceparent);

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        correlation_id = %correlation_id.as_deref().unwrap_or(""),
        trace_id = %trace_id.as_deref().unwrap_or(""),
        route = %route,
    );

    async move {
        let response = next.run(req).await;
        let status = response.status().as_u16().to_string();
        let duration = start.elapsed().as_secs_f64();
        let labels = &[
            ("method", method.as_str()),
            ("route", route.as_str()),
            ("status", status.as_str()),
        ];
        state.metrics.counter_inc("moqentra_http_requests_total", labels, 1);
        state
            .metrics
            .histogram_record("moqentra_http_request_duration_seconds", labels, duration);

        tracing::debug!(
            method = %method,
            route = %route,
            status = %status,
            duration_seconds = %duration,
            "handled request"
        );
        response
    }
    .instrument(span)
    .await
}

fn parse_traceparent(header: &str) -> Option<String> {
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() >= 2 && parts[0] == "00" {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Health check for the configured object storage backend.
#[derive(Clone)]
pub struct ObjectStorageHealthCheck {
    pub store: Arc<dyn ObjectStorage + Send + Sync>,
}

#[async_trait]
impl HealthCheck for ObjectStorageHealthCheck {
    fn name(&self) -> &str {
        "object_store"
    }

    fn required(&self) -> bool {
        true
    }

    async fn check(&self) -> Result<(), String> {
        let key = format!("moqentra/healthz-probe/{}", Uuid::new_v4());
        let data = Bytes::from_static(b"healthz");
        let _ = self
            .store
            .put_object(&key, data, None)
            .await
            .map_err(|e| format!("object store probe write failed: {e}"))?;
        self.store
            .delete_object(&key)
            .await
            .map_err(|e| format!("object store probe cleanup failed: {e}"))?;
        Ok(())
    }
}

/// Health check for the PostgreSQL connection pool when configured.
#[derive(Clone)]
pub struct DatabaseHealthCheck {
    pub pool: sqlx::PgPool,
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        "database"
    }

    fn required(&self) -> bool {
        true
    }

    async fn check(&self) -> Result<(), String> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| format!("database probe failed: {e}"))
    }
}

async fn whoami(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WhoAmIResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    Ok(Json(WhoAmIResponse {
        principal: ctx.principal,
        tenant_id: ctx.tenant_id.to_string(),
        request_id: ctx.request_id,
    }))
}

async fn compile_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResult>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::ApplicationVersion).await?;
    let mut result = state.compiler.compile(req.spec)?;
    let project_id = ctx.project_id.ok_or_else(|| {
        Error::invalid_argument("project context required to compile application")
    })?;
    result.resolved_artifact_bindings =
        resolve_artifact_bindings(&state, ctx.tenant_id, project_id, &result.artifact_bindings)?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "application.compile",
        Resource::ApplicationVersion,
        Some(&result.digest),
        AuditOutcome::Success,
    )
    .await;
    Ok(Json(result))
}

fn resolve_artifact_bindings(
    state: &AppState,
    tenant_id: moqentra_types::TenantId,
    project_id: moqentra_types::ProjectId,
    bindings: &std::collections::BTreeMap<String, moqentra_domain::application::ResourceRef>,
) -> Result<std::collections::BTreeMap<String, ArtifactBinding>, moqentra_types::Error> {
    use moqentra_domain::application::ResourceRef;
    use moqentra_domain::model_registry::ModelVersion;

    let mut resolved = std::collections::BTreeMap::new();
    for (slot, resource) in bindings {
        let binding = match resource {
            ResourceRef::Model(version_id) => {
                let version: ModelVersion = {
                    let models = state.models.lock().unwrap_or_else(|e| e.into_inner());
                    models.get_version(tenant_id, *version_id)?.clone()
                };
                let artifact = version
                    .artifacts
                    .iter()
                    .find(|a| a.media_type.contains("onnx"))
                    .or_else(|| version.artifacts.first())
                    .ok_or_else(|| {
                        moqentra_types::Error::invalid_argument(format!(
                            "model version {version_id} has no artifacts"
                        ))
                    })?;
                let object_key = ObjectKey::asset(
                    tenant_id,
                    project_id,
                    "models",
                    &version.model_id.to_string(),
                    &version.version,
                    &artifact.asset_id.to_string(),
                )?;
                ArtifactBinding {
                    object_key: object_key.to_string(),
                    digest: artifact.digest.clone(),
                    media_type: artifact.media_type.clone(),
                    size_bytes: artifact.size_bytes,
                }
            }
            ResourceRef::Dataset(version_id) => {
                let version = {
                    let datasets = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                    datasets.get_version(tenant_id, *version_id)?.clone()
                };
                let asset = version.assets.first().ok_or_else(|| {
                    moqentra_types::Error::invalid_argument(format!(
                        "dataset version {version_id} has no assets"
                    ))
                })?;
                ArtifactBinding {
                    object_key: asset.object_key.clone(),
                    digest: asset.digest.clone(),
                    media_type: asset.media_type.clone(),
                    size_bytes: asset.size,
                }
            }
            ResourceRef::Secret { .. }
            | ResourceRef::Stream { .. }
            | ResourceRef::Device { .. } => {
                // Secrets and streams are resolved at runner start, not materialized here.
                continue;
            }
        };
        resolved.insert(slot.clone(), binding);
    }
    Ok(resolved)
}

async fn create_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<(StatusCode, Json<DatasetResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::Dataset).await?;
    let project_id = resolve_project_id(&ctx, &req.project_id)?;

    let gen = RandomIdGenerator;
    let dataset = Dataset::new(DatasetId::new_v7(&gen), ctx.tenant_id, project_id, req.name)?;
    let created = {
        let mut registry = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        registry.create_dataset(dataset)?
    };
    emit_event(
        &state,
        ctx.tenant_id,
        "dataset",
        &created.id.to_string(),
        "dataset.created",
        serde_json::json!({"name": created.name}),
    )
    .await;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "dataset.create",
        Resource::Dataset,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(DatasetResponse {
            id: created.id.to_string(),
            tenant_id: created.tenant_id.to_string(),
            project_id: created.project_id.to_string(),
            name: created.name,
            state: format!("{:?}", created.state),
        }),
    ))
}

async fn get_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DatasetResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::Dataset).await?;

    let dataset_id = DatasetId::try_from(id.as_str())?;
    let registry = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
    let ds = registry.get_dataset(ctx.tenant_id, dataset_id)?;
    Ok(Json(DatasetResponse {
        id: ds.id.to_string(),
        tenant_id: ds.tenant_id.to_string(),
        project_id: ds.project_id.to_string(),
        name: ds.name.clone(),
        state: format!("{:?}", ds.state),
    }))
}

async fn list_datasets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageRequest>,
) -> Result<Json<Page<DatasetResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::List, Resource::Dataset).await?;

    let registry = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
    let mut items: Vec<Dataset> = registry
        .list_datasets(ctx.tenant_id, ctx.project_id)
        .into_iter()
        .cloned()
        .collect();
    items.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at).then(a.id.to_string().cmp(&b.id.to_string()))
    });

    let total = items.len() as u64;
    let limit = page.bounded_limit() as usize;
    let offset = page.offset as usize;
    let end = (offset + limit).min(items.len());
    let page_items = items[offset..end]
        .iter()
        .map(|ds| DatasetResponse {
            id: ds.id.to_string(),
            tenant_id: ds.tenant_id.to_string(),
            project_id: ds.project_id.to_string(),
            name: ds.name.clone(),
            state: format!("{:?}", ds.state),
        })
        .collect();
    Ok(Json(Page::new(page_items, total, page)))
}

#[derive(Deserialize)]
struct CreateExperimentRequest {
    name: String,
    project_id: String,
    dataset_version_id: String,
    target_metric: String,
}

#[derive(Serialize, Deserialize)]
struct ExperimentResponse {
    id: String,
    name: String,
    state: String,
}

#[derive(Deserialize)]
struct CreateTrainingJobRequest {
    experiment_id: String,
    project_id: String,
    code_digest: String,
    image_digest: String,
    dataset_version_id: String,
    argv: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct TrainingJobResponse {
    id: String,
    experiment_id: String,
    state: String,
}

#[derive(Deserialize)]
struct CreateModelRequest {
    name: String,
    project_id: String,
}

#[derive(Serialize, Deserialize)]
struct ModelResponse {
    id: String,
    name: String,
}

impl ModelResponse {
    fn from_model(m: &Model) -> Self {
        Self {
            id: m.id.to_string(),
            name: m.name.clone(),
        }
    }
}

#[derive(Serialize)]
struct ModelVersionResponse {
    id: String,
    model_id: String,
    version: String,
    state: String,
    created_at: String,
    updated_at: String,
}

impl ModelVersionResponse {
    fn from_version(v: &ModelVersion) -> Self {
        Self {
            id: v.id.to_string(),
            model_id: v.model_id.to_string(),
            version: v.version.clone(),
            state: format!("{:?}", v.state),
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}

#[derive(Serialize)]
struct ConversionJobResponse {
    id: String,
    source_model_version_id: String,
    target: String,
    state: String,
    cache_key: String,
}

impl ConversionJobResponse {
    fn from_job(job: &ConversionJob) -> Self {
        Self {
            id: job.id.to_string(),
            source_model_version_id: job.source_model_version_id.to_string(),
            target: job.target.to_string(),
            state: job.state.to_string(),
            cache_key: job.cache_key.clone(),
        }
    }
}

#[derive(Deserialize)]
struct ConvertModelVersionRequest {
    profile: ConversionProfile,
    parameters: Option<BTreeMap<String, String>>,
}

#[derive(Serialize)]
struct EvaluationRunResponse {
    id: String,
    model_version_id: String,
    dataset_version_id: String,
    state: String,
}

impl EvaluationRunResponse {
    fn from_run(run: &EvaluationRun) -> Self {
        Self {
            id: run.id.to_string(),
            model_version_id: run.model_version_id.to_string(),
            dataset_version_id: run.dataset_version_id.to_string(),
            state: run.state.to_string(),
        }
    }
}

#[derive(Deserialize)]
struct EvaluateModelVersionRequest {
    dataset_version_id: String,
    seed: u64,
    hardware_profile: String,
    preprocess_version: String,
    postprocess_version: String,
}

#[derive(Deserialize)]
struct CreateAnnotationProjectRequest {
    name: String,
    project_id: String,
    dataset_version_id: String,
    task_type: String,
}

#[derive(Serialize, Deserialize)]
struct AnnotationProjectResponse {
    id: String,
    name: String,
    state: String,
}

async fn create_experiment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateExperimentRequest>,
) -> Result<(StatusCode, Json<ExperimentResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::TrainingJob).await?;
    let project_id = resolve_project_id(&ctx, &req.project_id)?;
    let gen = RandomIdGenerator;
    let exp = Experiment::new(
        ExperimentId::new_v7(&gen),
        ctx.tenant_id,
        project_id,
        req.name,
        DatasetVersionId::try_from(req.dataset_version_id.as_str())?,
        req.target_metric,
    )?;
    let created = {
        let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_experiment(exp)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Training,
        "experiment.create",
        Resource::TrainingJob,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(ExperimentResponse {
            id: created.id.to_string(),
            name: created.name,
            state: "active".to_string(),
        }),
    ))
}

async fn list_experiments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageRequest>,
) -> Result<Json<Page<ExperimentResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::List, Resource::TrainingJob).await?;
    let reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
    let mut items: Vec<Experiment> = reg
        .list_experiments(ctx.tenant_id, ctx.project_id)
        .into_iter()
        .cloned()
        .collect();
    items.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at).then(a.id.to_string().cmp(&b.id.to_string()))
    });
    let total = items.len() as u64;
    let limit = page.bounded_limit() as usize;
    let offset = page.offset as usize;
    let end = (offset + limit).min(items.len());
    let page_items = items[offset..end]
        .iter()
        .map(|e| ExperimentResponse {
            id: e.id.to_string(),
            name: e.name.clone(),
            state: "active".to_string(),
        })
        .collect();
    Ok(Json(Page::new(page_items, total, page)))
}

async fn create_training_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTrainingJobRequest>,
) -> Result<(StatusCode, Json<TrainingJobResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::TrainingJob).await?;
    let project_id = resolve_project_id(&ctx, &req.project_id)?;
    let gen = RandomIdGenerator;
    let spec = TrainingJobSpec {
        code_digest: req.code_digest,
        image_digest: req.image_digest,
        dataset_version_id: DatasetVersionId::try_from(req.dataset_version_id.as_str())?,
        hyperparameters: ParameterSchema {
            argv: req.argv,
            env: Default::default(),
            config_files: Default::default(),
        },
        seed: 42,
        resources: ResourceRequest {
            replicas: 1,
            cpu_milli: 1000,
            memory_mib: 4096,
            ephemeral_storage_mib: 10240,
            accelerator_kind: None,
            accelerator_count: 0,
            topology: None,
        },
        distributed: DistributedConfig::Single,
        max_attempts: 3,
        deadline_seconds: 3600,
    };
    let job = TrainingJob::new(
        TrainingJobId::new_v7(&gen),
        ExperimentId::try_from(req.experiment_id.as_str())?,
        ctx.tenant_id,
        project_id,
        spec,
    )?;
    let created = {
        let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_job(job)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Training,
        "training_job.create",
        Resource::TrainingJob,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    emit_event(
        &state,
        ctx.tenant_id,
        "training_job",
        &created.id.to_string(),
        "training_job.queued",
        serde_json::json!({
            "job_id": created.id.to_string(),
            "tenant_id": ctx.tenant_id.to_string(),
            "project_id": project_id.to_string(),
            "experiment_id": created.experiment_id.to_string(),
            "priority": 10
        }),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(TrainingJobResponse {
            id: created.id.to_string(),
            experiment_id: created.experiment_id.to_string(),
            state: format!("{:?}", created.state),
        }),
    ))
}

async fn list_training_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageRequest>,
) -> Result<Json<Page<TrainingJobResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::TrainingJob).await?;
    let reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
    let mut items: Vec<TrainingJob> =
        reg.list_jobs(ctx.tenant_id, None).into_iter().cloned().collect();
    items.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at).then(a.id.to_string().cmp(&b.id.to_string()))
    });
    let total = items.len() as u64;
    let limit = page.bounded_limit() as usize;
    let offset = page.offset as usize;
    let end = (offset + limit).min(items.len());
    let page_items = items[offset..end]
        .iter()
        .map(|j| TrainingJobResponse {
            id: j.id.to_string(),
            experiment_id: j.experiment_id.to_string(),
            state: format!("{:?}", j.state),
        })
        .collect();
    Ok(Json(Page::new(page_items, total, page)))
}

async fn admit_training_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TrainingJobResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Execute, Resource::TrainingJob).await?;
    let job_id = TrainingJobId::try_from(id.as_str())?;
    let job = {
        let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
        reg.admit_job(ctx.tenant_id, job_id)?.clone()
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Training,
        "training_job.admit",
        Resource::TrainingJob,
        Some(&job.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    emit_event(
        &state,
        ctx.tenant_id,
        "training_job",
        &job.id.to_string(),
        "training_job.admitted",
        serde_json::json!({
            "job_id": job.id.to_string(),
            "tenant_id": ctx.tenant_id.to_string(),
            "cpu_cores": 2,
            "memory_mib": 4096
        }),
    )
    .await;
    Ok(Json(TrainingJobResponse {
        id: job.id.to_string(),
        experiment_id: job.experiment_id.to_string(),
        state: format!("{:?}", job.state),
    }))
}

async fn cancel_training_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TrainingJobResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Execute, Resource::TrainingJob).await?;
    let job_id = TrainingJobId::try_from(id.as_str())?;
    let job = {
        let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
        reg.cancel_job(ctx.tenant_id, job_id)?.clone()
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Training,
        "training_job.cancel",
        Resource::TrainingJob,
        Some(&job.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok(Json(TrainingJobResponse {
        id: job.id.to_string(),
        experiment_id: job.experiment_id.to_string(),
        state: format!("{:?}", job.state),
    }))
}

#[derive(Deserialize)]
struct CreateDatasetVersionRequest {
    dataset_id: String,
}

#[derive(Deserialize)]
struct AddAssetRequest {
    name: String,
    object_key: String,
    digest: String,
    size: u64,
    media_type: String,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GenerateSplitsRequest {
    seed: u64,
    train: f64,
    val: f64,
    test: f64,
}

#[derive(Serialize, Deserialize)]
struct DatasetVersionResponse {
    id: String,
    dataset_id: String,
    state: String,
    manifest_digest: Option<String>,
}

async fn create_dataset_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateDatasetVersionRequest>,
) -> Result<(StatusCode, Json<DatasetVersionResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::DatasetVersion).await?;
    let dataset_id = DatasetId::try_from(req.dataset_id.as_str())?;
    // Ensure dataset exists and belongs to tenant.
    {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        let _ = reg.get_dataset(ctx.tenant_id, dataset_id)?;
    }
    let gen = RandomIdGenerator;
    let project_id = ctx.project_id.ok_or_else(|| {
        Error::invalid_argument("X-Project-Id is required when creating a dataset version")
    })?;
    let version = DatasetVersion::new(
        DatasetVersionId::new_v7(&gen),
        dataset_id,
        ctx.tenant_id,
        project_id,
    );
    let created = {
        let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_version(version)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "dataset_version.create",
        Resource::DatasetVersion,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(DatasetVersionResponse {
            id: created.id.to_string(),
            dataset_id: created.dataset_id.to_string(),
            state: format!("{:?}", created.state),
            manifest_digest: created.manifest_digest.clone(),
        }),
    ))
}

async fn add_dataset_version_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(version_id): axum::extract::Path<String>,
    Json(req): Json<AddAssetRequest>,
) -> Result<(StatusCode, Json<DatasetVersionResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::DatasetVersion).await?;
    let version_id = DatasetVersionId::try_from(version_id.as_str())?;
    let metadata = req.metadata.unwrap_or(serde_json::Value::Null);
    let gen = RandomIdGenerator;
    let asset = AssetRef {
        id: AssetId::new_v7(&gen),
        name: req.name,
        object_key: req.object_key,
        digest: req.digest,
        size: req.size,
        media_type: req.media_type,
        metadata,
    };
    let updated = {
        let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.add_asset(ctx.tenant_id, version_id, asset)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "dataset_version.add_asset",
        Resource::DatasetVersion,
        Some(&updated.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::OK,
        Json(DatasetVersionResponse {
            id: updated.id.to_string(),
            dataset_id: updated.dataset_id.to_string(),
            state: format!("{:?}", updated.state),
            manifest_digest: updated.manifest_digest.clone(),
        }),
    ))
}

async fn generate_dataset_version_splits(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(version_id): axum::extract::Path<String>,
    Json(req): Json<GenerateSplitsRequest>,
) -> Result<(StatusCode, Json<DatasetVersionResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::DatasetVersion).await?;
    let version_id = DatasetVersionId::try_from(version_id.as_str())?;
    let updated = {
        let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.generate_splits(
            ctx.tenant_id,
            version_id,
            req.seed,
            req.train,
            req.val,
            req.test,
        )?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "dataset_version.generate_splits",
        Resource::DatasetVersion,
        Some(&updated.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::OK,
        Json(DatasetVersionResponse {
            id: updated.id.to_string(),
            dataset_id: updated.dataset_id.to_string(),
            state: format!("{:?}", updated.state),
            manifest_digest: updated.manifest_digest.clone(),
        }),
    ))
}

async fn publish_dataset_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(version_id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<DatasetVersionResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::DatasetVersion).await?;
    let version_id = DatasetVersionId::try_from(version_id.as_str())?;

    // Gather assets without holding the registry across await points.
    let (tenant_id, assets) = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        let v = reg.get_version(ctx.tenant_id, version_id)?;
        (
            v.tenant_id,
            v.assets
                .iter()
                .map(|a| {
                    (
                        a.id.to_string(),
                        a.name.clone(),
                        a.object_key.clone(),
                        a.media_type.clone(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    };

    let mut failures: Vec<(String, String, String)> = Vec::new();
    for (id, _name, key, media_type) in &assets {
        match state.object_store.get_object(key).await {
            Ok((bytes, _)) => {
                if let Err(e) = state.media_validator.validate(&bytes, media_type).await {
                    failures.push((id.clone(), _name.clone(), e.to_string()));
                }
            }
            Err(e) => failures.push((id.clone(), _name.clone(), e.to_string())),
        }
    }

    let updated = {
        let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        for (id, _name, reason) in &failures {
            reg.mark_asset_failed(tenant_id, version_id, id, reason)?;
        }
        for (id, _name, _, _) in &assets {
            if !failures.iter().any(|(fid, _, _)| fid == id) {
                reg.mark_asset_valid(tenant_id, version_id, id)?;
            }
        }
        if !failures.is_empty() {
            return Err(Error::invalid_argument(format!(
                "asset validation failed: {}",
                failures.iter().map(|(_, _, r)| r.as_str()).collect::<Vec<_>>().join(", ")
            ))
            .into());
        }
        reg.publish_version(tenant_id, version_id)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "dataset_version.publish",
        Resource::DatasetVersion,
        Some(&updated.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::OK,
        Json(DatasetVersionResponse {
            id: updated.id.to_string(),
            dataset_id: updated.dataset_id.to_string(),
            state: format!("{:?}", updated.state),
            manifest_digest: updated.manifest_digest.clone(),
        }),
    ))
}

#[derive(Deserialize)]
struct CreateUploadSessionRequest {
    resource_type: String,
    resource_id: String,
    version_id: String,
    name: String,
    media_type: String,
    part_size: u64,
    total_size: u64,
    ttl_seconds: Option<u64>,
}

#[derive(Serialize)]
struct UploadPartInfo {
    part_number: i32,
    size: u64,
    completed: bool,
}

#[derive(Serialize)]
struct UploadSessionResponse {
    id: String,
    target_key: String,
    media_type: String,
    part_size: u64,
    total_size: u64,
    parts: Vec<UploadPartInfo>,
    state: String,
    expires_at: String,
}

impl From<&UploadSession> for UploadSessionResponse {
    fn from(s: &UploadSession) -> Self {
        Self {
            id: s.id.clone(),
            target_key: s.target_key.as_str().to_string(),
            media_type: s.media_type.clone(),
            part_size: s.part_size,
            total_size: s.total_size,
            parts: s
                .parts
                .values()
                .map(|p| UploadPartInfo {
                    part_number: p.part_number,
                    size: p.size,
                    completed: p.completed,
                })
                .collect(),
            state: format!("{:?}", s.state),
            expires_at: s.expires_at.to_string(),
        }
    }
}

#[derive(Deserialize)]
struct UploadPartQuery {
    sig: Option<String>,
    expires: Option<u64>,
}

#[derive(Serialize)]
struct PartUploadUrl {
    part_number: i32,
    upload_url: String,
    expires_at: String,
}

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

fn sign_part_upload(secret: &str, session_id: &str, part_number: i32, expires_at: u64) -> String {
    use hmac::Mac;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take a key of any size");
    mac.update(format!("{}:{}:{}", session_id, part_number, expires_at).as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn verify_part_upload(
    secret: &str,
    session_id: &str,
    part_number: i32,
    expires_at: u64,
    sig_hex: &str,
) -> Result<(), Error> {
    use hmac::Mac;
    let tag = hex::decode(sig_hex)
        .map_err(|_| Error::unauthenticated("invalid part upload signature"))?;
    let now = UtcTimestamp::now().as_offset().unix_timestamp() as u64;
    if now > expires_at {
        return Err(Error::unauthenticated("signed upload URL has expired"));
    }
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take a key of any size");
    mac.update(format!("{}:{}:{}", session_id, part_number, expires_at).as_bytes());
    mac.verify_slice(&tag)
        .map_err(|_| Error::unauthenticated("invalid part upload signature"))
}

async fn create_upload_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUploadSessionRequest>,
) -> Result<(StatusCode, Json<UploadSessionResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::DatasetVersion).await?;
    let project_id = ctx.project_id.ok_or_else(|| {
        Error::invalid_argument("X-Project-Id is required when creating an upload session")
    })?;
    let ttl = req.ttl_seconds.unwrap_or(3600).clamp(60, 86400);
    let target_key = ObjectKey::asset(
        ctx.tenant_id,
        project_id,
        &req.resource_type,
        &req.resource_id,
        &req.version_id,
        &req.name,
    )?;
    let session_id = Uuid::new_v4().to_string();
    let backend_upload_id = state
        .object_store
        .start_multipart(target_key.as_str(), Some(&req.media_type))
        .await?;
    let session = UploadSession::new(
        session_id,
        ctx.tenant_id,
        project_id,
        target_key,
        req.media_type,
        req.part_size,
        req.total_size,
        ttl,
    )?;
    let mut session = session;
    session.backend_upload_id = Some(backend_upload_id);
    state.upload_sessions.save(&session).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "upload_session.create",
        Resource::DatasetVersion,
        Some(&session.id),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(UploadSessionResponse::from(&session)),
    ))
}

async fn get_upload_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<UploadSessionResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::DatasetVersion).await?;
    let session = state
        .upload_sessions
        .get(&session_id)
        .await?
        .ok_or_else(|| Error::not_found("upload session"))?;
    if session.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("upload session").into());
    }
    Ok(Json(UploadSessionResponse::from(&session)))
}

async fn list_upload_session_parts(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<UploadSessionResponse>, ApiError> {
    get_upload_session(State(state), headers, axum::extract::Path(session_id)).await
}

async fn list_part_upload_urls(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<Vec<PartUploadUrl>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::DatasetVersion).await?;
    let session = state
        .upload_sessions
        .get(&session_id)
        .await?
        .ok_or_else(|| Error::not_found("upload session"))?;
    if session.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("upload session").into());
    }
    let ttl_seconds = 900_u64; // short-lived signed URLs
    let max_expires = (UtcTimestamp::now().as_offset().unix_timestamp() as u64) + ttl_seconds;
    let session_expires = (session.expires_at.as_offset().unix_timestamp() as u64).min(max_expires);
    let urls: Vec<PartUploadUrl> = session
        .parts
        .values()
        .filter(|p| !p.completed)
        .map(|p| {
            let n = p.part_number;
            let sig = sign_part_upload(&state.upload_sig_secret, &session_id, n, session_expires);
            PartUploadUrl {
                part_number: n,
                upload_url: format!(
                    "/v1/upload-sessions/{}/parts/{}?sig={}&expires={}",
                    session_id, n, sig, session_expires
                ),
                expires_at: session_expires.to_string(),
            }
        })
        .collect();
    Ok(Json(urls))
}

async fn upload_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path((session_id, part_number)): axum::extract::Path<(String, i32)>,
    axum::extract::Query(q): axum::extract::Query<UploadPartQuery>,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::DatasetVersion).await?;
    let mut session = state
        .upload_sessions
        .get(&session_id)
        .await?
        .ok_or_else(|| Error::not_found("upload session"))?;
    if session.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("upload session").into());
    }
    if let (Some(sig), Some(expires)) = (q.sig, q.expires) {
        verify_part_upload(
            &state.upload_sig_secret,
            &session_id,
            part_number,
            expires,
            &sig,
        )?;
    }
    let expected_size = session
        .parts
        .get(&part_number)
        .map(|p| p.size)
        .ok_or_else(|| Error::invalid_argument("invalid part number"))?;
    let actual_size = body.len() as u64;
    if actual_size != expected_size {
        return Err(Error::invalid_argument(format!(
            "part {} size mismatch: expected {}, got {}",
            part_number, expected_size, actual_size
        ))
        .into());
    }
    let backend_upload_id = session
        .backend_upload_id
        .clone()
        .ok_or_else(|| Error::internal("upload session has no backend id"))?;
    let etag = state
        .object_store
        .upload_part(
            session.target_key.as_str(),
            &backend_upload_id,
            part_number,
            body.clone(),
        )
        .await?;
    let digest = part_digest(&body);
    session.mark_part_uploaded(part_number, etag, digest)?;
    state.upload_sessions.save(&session).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "upload_session.upload_part",
        Resource::DatasetVersion,
        Some(&session.id),
        AuditOutcome::Success,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn complete_upload_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<UploadSessionResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::DatasetVersion).await?;
    let mut session = state
        .upload_sessions
        .get(&session_id)
        .await?
        .ok_or_else(|| Error::not_found("upload session"))?;
    if session.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("upload session").into());
    }
    session.validate_for_completion()?;
    let backend_upload_id = session
        .backend_upload_id
        .clone()
        .ok_or_else(|| Error::internal("upload session has no backend id"))?;
    let parts: Vec<(i32, String)> = session
        .parts
        .values()
        .map(|p| (p.part_number, p.etag.clone().unwrap_or_default()))
        .collect();
    let _meta = state
        .object_store
        .complete_multipart(session.target_key.as_str(), &backend_upload_id, parts)
        .await?;
    session.state = moqentra_object_store::upload_session::UploadSessionState::Completed;
    state.upload_sessions.save(&session).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "upload_session.complete",
        Resource::DatasetVersion,
        Some(&session.id),
        AuditOutcome::Success,
    )
    .await;
    Ok(Json(UploadSessionResponse::from(&session)))
}

async fn abort_upload_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Delete, Resource::DatasetVersion).await?;
    let session = state
        .upload_sessions
        .get(&session_id)
        .await?
        .ok_or_else(|| Error::not_found("upload session"))?;
    if session.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("upload session").into());
    }
    if let Some(backend_upload_id) = session.backend_upload_id {
        let _ = state
            .object_store
            .abort_multipart(session.target_key.as_str(), &backend_upload_id)
            .await;
    }
    state.upload_sessions.delete(&session_id).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "upload_session.abort",
        Resource::DatasetVersion,
        Some(&session_id),
        AuditOutcome::Success,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateImportJobRequest {
    source_url: String,
    target_key: String,
    media_type: String,
    total_bytes: u64,
    #[serde(default = "default_import_concurrency")]
    concurrency: u32,
    #[serde(default = "default_import_deadline")]
    deadline_seconds: u32,
}

fn default_import_concurrency() -> u32 {
    1
}

fn default_import_deadline() -> u32 {
    300
}

#[derive(Serialize)]
struct ImportJobResponse {
    id: String,
    state: String,
    source_url: String,
    target_key: String,
    media_type: String,
    total_bytes: u64,
    transferred_bytes: u64,
    concurrency: u32,
    deadline_seconds: u32,
    digest: Option<String>,
    failure: Option<String>,
    retry_count: u32,
}

impl From<&ImportJob> for ImportJobResponse {
    fn from(j: &ImportJob) -> Self {
        Self {
            id: j.id.clone(),
            state: format!("{:?}", j.state),
            source_url: j.source_url.clone(),
            target_key: j.target_key.clone(),
            media_type: j.media_type.clone(),
            total_bytes: j.total_bytes,
            transferred_bytes: j.transferred_bytes,
            concurrency: j.concurrency,
            deadline_seconds: j.deadline_seconds,
            digest: j.digest.clone(),
            failure: j.failure.as_ref().map(|f| format!("{:?}", f)),
            retry_count: j.retry_count,
        }
    }
}

async fn create_import_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateImportJobRequest>,
) -> Result<(StatusCode, Json<ImportJobResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::DatasetVersion).await?;
    let project_id = ctx.project_id.ok_or_else(|| {
        Error::invalid_argument("X-Project-Id is required when creating an import job")
    })?;
    ObjectKey::from_str(ctx.tenant_id, project_id, &req.target_key)?;
    let id = Uuid::new_v4().to_string();
    let job = ImportJob::new_v1(
        id.clone(),
        ctx.tenant_id,
        project_id,
        req.source_url,
        req.target_key,
        req.media_type,
        req.total_bytes,
        req.concurrency.clamp(1, 16),
        req.deadline_seconds.clamp(30, 3600),
    )?;
    state.import_jobs.save(&job).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "import_job.create",
        Resource::DatasetVersion,
        Some(&id),
        AuditOutcome::Success,
    )
    .await;
    Ok((StatusCode::CREATED, Json(ImportJobResponse::from(&job))))
}

async fn get_import_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<Json<ImportJobResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::DatasetVersion).await?;
    let job = state
        .import_jobs
        .get(&job_id)
        .await?
        .ok_or_else(|| Error::not_found("import job"))?;
    if job.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("import job").into());
    }
    Ok(Json(ImportJobResponse::from(&job)))
}

async fn cancel_import_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Delete, Resource::DatasetVersion).await?;
    let mut job = state
        .import_jobs
        .get(&job_id)
        .await?
        .ok_or_else(|| Error::not_found("import job"))?;
    if job.tenant_id != ctx.tenant_id {
        return Err(Error::not_found("import job").into());
    }
    job.cancel()?;
    state.import_jobs.save(&job).await?;
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "import_job.cancel",
        Resource::DatasetVersion,
        Some(&job_id),
        AuditOutcome::Success,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<ModelResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::ModelVersion).await?;
    let project_id = resolve_project_id(&ctx, &req.project_id)?;
    let gen = RandomIdGenerator;
    let model = Model::new(ModelId::new_v7(&gen), ctx.tenant_id, project_id, req.name)?;
    let created = {
        let mut reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_model(model)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::ModelPublish,
        "model.create",
        Resource::ModelVersion,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(ModelResponse {
            id: created.id.to_string(),
            name: created.name,
        }),
    ))
}

async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageRequest>,
) -> Result<Json<Page<ModelResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::List, Resource::ModelVersion).await?;
    let reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
    let mut items: Vec<Model> =
        reg.list_models(ctx.tenant_id, ctx.project_id).into_iter().cloned().collect();
    items.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at).then(a.id.to_string().cmp(&b.id.to_string()))
    });
    let total = items.len() as u64;
    let limit = page.bounded_limit() as usize;
    let offset = page.offset as usize;
    let end = (offset + limit).min(items.len());
    let page_items = items[offset..end].iter().map(ModelResponse::from_model).collect();
    Ok(Json(Page::new(page_items, total, page)))
}

async fn request_publish_model_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ModelVersionResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::ModelVersion).await?;
    let version_id = ModelVersionId::try_from(id.as_str())?;
    let mut reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
    let version = reg
        .with_version_mut(ctx.tenant_id, version_id, |v| {
            ArtifactReconciler::reconcile(v)?;
            Ok(v.clone())
        })
        .map_err(ApiError::from)?;
    Ok(Json(ModelVersionResponse::from_version(&version)))
}

async fn approve_model_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ModelVersionResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Admin, Resource::ModelVersion).await?;
    let user_id = match ctx.principal {
        Principal::User { id } => id,
        _ => return Err(Error::permission_denied("approval requires a user principal").into()),
    };
    let version_id = ModelVersionId::try_from(id.as_str())?;
    let version = {
        let mut reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
        reg.with_version_mut(ctx.tenant_id, version_id, |v| {
            v.approve(user_id)?;
            Ok(v.clone())
        })
        .map_err(ApiError::from)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::ModelPublish,
        "model-version.approve",
        Resource::ModelVersion,
        Some(&version.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok(Json(ModelVersionResponse::from_version(&version)))
}

async fn convert_model_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ConvertModelVersionRequest>,
) -> Result<(StatusCode, Json<ConversionJobResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::ModelVersion).await?;
    let version_id = ModelVersionId::try_from(id.as_str())?;
    let project_id = ctx.project_id.ok_or_else(|| {
        ApiError::from(Error::invalid_argument(
            "project_id is required for conversion",
        ))
    })?;

    // Verify the source model version exists within the tenant/project.
    {
        let reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
        let _ = reg.get_version(ctx.tenant_id, version_id).map_err(ApiError::from)?;
    }

    let gen = RandomIdGenerator;
    let job = ConversionJob::new(
        ConversionJobId::new_v7(&gen),
        ctx.tenant_id,
        project_id,
        version_id,
        req.profile.target,
        req.profile,
        req.parameters.unwrap_or_default(),
    )?;
    let created = {
        let mut reg = state.conversions.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_job(job)?
    };
    Ok((
        StatusCode::CREATED,
        Json(ConversionJobResponse::from_job(&created)),
    ))
}

async fn evaluate_model_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<EvaluateModelVersionRequest>,
) -> Result<(StatusCode, Json<EvaluationRunResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::ModelVersion).await?;
    let version_id = ModelVersionId::try_from(id.as_str())?;
    let project_id = ctx.project_id.ok_or_else(|| {
        ApiError::from(Error::invalid_argument(
            "project_id is required for evaluation",
        ))
    })?;

    {
        let reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
        let _ = reg.get_version(ctx.tenant_id, version_id).map_err(ApiError::from)?;
    }

    let gen = RandomIdGenerator;
    let run = EvaluationRun::new(
        EvaluationRunId::new_v7(&gen),
        ctx.tenant_id,
        project_id,
        version_id,
        DatasetVersionId::try_from(req.dataset_version_id.as_str())?,
        req.seed,
        req.hardware_profile,
        req.preprocess_version,
        req.postprocess_version,
    )?;
    let created = {
        let mut reg = state.evaluations.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_run(run)?
    };
    Ok((
        StatusCode::CREATED,
        Json(EvaluationRunResponse::from_run(&created)),
    ))
}

async fn create_annotation_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateAnnotationProjectRequest>,
) -> Result<(StatusCode, Json<AnnotationProjectResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::AnnotationTask).await?;
    let project_id = resolve_project_id(&ctx, &req.project_id)?;
    let task_type = match req.task_type.as_str() {
        "boundingBox" | "rectTool" => TaskType::BoundingBox,
        "polygon" | "polygonTool" => TaskType::Polygon,
        "classification" | "imageClassification" => TaskType::ImageClassification,
        other => return Err(Error::invalid_argument(format!("unknown task_type: {other}")).into()),
    };
    let gen = RandomIdGenerator;
    let ap = AnnotationProject::new(
        AnnotationProjectId::new_v7(&gen),
        ctx.tenant_id,
        project_id,
        DatasetVersionId::try_from(req.dataset_version_id.as_str())?,
        req.name,
        task_type,
        Ontology::new(vec![], vec![req.task_type]),
    )?;
    let created = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_project(ap)?
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_project.create",
        Resource::AnnotationTask,
        Some(&created.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(AnnotationProjectResponse {
            id: created.id.to_string(),
            name: created.name,
            state: format!("{:?}", created.state),
        }),
    ))
}

async fn activate_annotation_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AnnotationProjectResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;
    let id = AnnotationProjectId::try_from(id.as_str())?;
    let p = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.activate(ctx.tenant_id, id)?.clone()
    };
    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_project.activate",
        Resource::AnnotationTask,
        Some(&p.id.to_string()),
        AuditOutcome::Success,
    )
    .await;
    Ok(Json(AnnotationProjectResponse {
        id: p.id.to_string(),
        name: p.name.clone(),
        state: format!("{:?}", p.state),
    }))
}

async fn list_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<PageRequest>,
) -> Result<Json<Page<serde_json::Value>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AuditLog).await?;
    let limit = page.bounded_limit().min(1000);
    let mut pending = state.outbox.poll_pending(limit).await?;
    pending.retain(|e| e.tenant_id == ctx.tenant_id);
    pending.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at).then(a.id.to_string().cmp(&b.id.to_string()))
    });
    let total = pending.len() as u64;
    let offset = page.offset as usize;
    let end = (offset + page.bounded_limit() as usize).min(pending.len());
    let items = pending[offset..end]
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "event_type": e.event_type,
                "aggregate_type": e.aggregate_type,
                "aggregate_id": e.aggregate_id,
                "status": e.status.to_string(),
            })
        })
        .collect();
    Ok(Json(Page::new(items, total, page)))
}

/// Global security response headers.
async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert(
        "content-security-policy",
        "default-src 'self'".parse().unwrap(),
    );
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    response
}

/// Rejects requests without an Authorization header for protected routes when
/// authentication is required. Health/ready endpoints stay unauthenticated;
/// the handler-layer `resolve_context` performs full token validation.
async fn require_auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if state.require_auth
        && !matches!(path, "/healthz" | "/readyz" | "/livez" | "/metrics")
        && bearer_token(req.headers()).is_none()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ProblemDetails::from_error(
                &Error::unauthenticated("authorization required"),
                "",
            )),
        )
            .into_response();
    }
    next.run(req).await
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/livez", get(livez))
        .route("/metrics", get(metrics))
        .route("/v1/whoami", get(whoami))
        .route("/v1/applications:compile", post(compile_application))
        .route("/v1/datasets", post(create_dataset).get(list_datasets))
        .route("/v1/datasets/{id}", get(get_dataset))
        .route(
            "/v1/experiments",
            post(create_experiment).get(list_experiments),
        )
        .route(
            "/v1/training-jobs",
            post(create_training_job).get(list_training_jobs),
        )
        .route("/v1/training-jobs/{id}/admit", post(admit_training_job))
        .route("/v1/training-jobs/{id}/cancel", post(cancel_training_job))
        .route("/v1/dataset-versions", post(create_dataset_version))
        .route(
            "/v1/dataset-versions/{id}/assets",
            post(add_dataset_version_asset),
        )
        .route(
            "/v1/dataset-versions/{id}/splits",
            post(generate_dataset_version_splits),
        )
        .route(
            "/v1/dataset-versions/{id}/publish",
            post(publish_dataset_version),
        )
        .route("/v1/upload-sessions", post(create_upload_session))
        .route(
            "/v1/upload-sessions/{id}",
            get(get_upload_session).delete(abort_upload_session),
        )
        .route(
            "/v1/upload-sessions/{id}/parts",
            get(list_upload_session_parts),
        )
        .route(
            "/v1/upload-sessions/{id}/part-urls",
            get(list_part_upload_urls),
        )
        .route(
            "/v1/upload-sessions/{id}/parts/{partNumber}",
            post(upload_part),
        )
        .route(
            "/v1/upload-sessions/{id}/complete",
            post(complete_upload_session),
        )
        .route("/v1/import-jobs", post(create_import_job))
        .route(
            "/v1/import-jobs/{id}",
            get(get_import_job).delete(cancel_import_job),
        )
        .route("/v1/models", post(create_model).get(list_models))
        .route(
            "/v1/model-versions/{id}/request-publish",
            post(request_publish_model_version),
        )
        .route(
            "/v1/model-versions/{id}/approve",
            post(approve_model_version),
        )
        .route(
            "/v1/model-versions/{id}/convert",
            post(convert_model_version),
        )
        .route(
            "/v1/model-versions/{id}/evaluate",
            post(evaluate_model_version),
        )
        .route("/v1/annotation-projects", post(create_annotation_project))
        .route(
            "/v1/annotation-projects/{id}/activate",
            post(activate_annotation_project),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks",
            post(crate::annotation::create_tasks).get(crate::annotation::list_tasks),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}",
            get(crate::annotation::get_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/assign",
            post(crate::annotation::assign_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/start",
            post(crate::annotation::start_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/submit",
            post(crate::annotation::submit_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/return",
            post(crate::annotation::return_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/approve",
            post(crate::annotation::approve_task),
        )
        .route(
            "/v1/annotation-projects/{id}/tasks/{taskId}/annotations",
            post(crate::annotation::autosave_annotation).get(crate::annotation::list_annotations),
        )
        .route(
            "/v1/annotation-projects/{id}/export-coco",
            get(crate::annotation::export_coco),
        )
        .route(
            "/v1/annotation-projects/{id}/import-coco",
            post(crate::annotation::import_coco),
        )
        .route(
            "/v1/annotation-projects/{id}/export-labelu",
            get(crate::annotation::export_labelu),
        )
        .route(
            "/v1/annotation-projects/{id}/import-labelu",
            post(crate::annotation::import_labelu),
        )
        .route(
            "/v1/annotation-projects/{id}/export-platform",
            get(crate::annotation::export_platform),
        )
        .route(
            "/v1/annotation-projects/{id}/import-platform",
            post(crate::annotation::import_platform),
        )
        .route(
            "/v1/assets/{assetId}/media-url",
            get(crate::annotation::get_asset_media_url),
        )
        .route("/v1/outbox", get(list_outbox))
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn_with_state(
            state,
            observability_middleware,
        ))
}

/// Background worker: poll outbox and apply side-effects (scheduler enqueue,
/// local admit/allocate when downstream URLs are configured or local-only).
pub fn spawn_outbox_dispatcher(state: AppState) {
    tokio::spawn(async move {
        loop {
            let pending = match state.outbox.poll_pending(16).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "outbox poll failed");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            };
            for event in pending {
                let action = plan_dispatch(&event.event_type, &event.payload);
                let result = apply_dispatch(&state, action).await;
                match result {
                    Ok(()) => {
                        let _ = state.outbox.mark_completed(event.id).await;
                    }
                    Err(msg) => {
                        tracing::warn!(
                            event_id = %event.id,
                            event_type = %event.event_type,
                            error = %msg,
                            "outbox dispatch failed"
                        );
                        let _ = state.outbox.mark_failed(event.id, msg).await;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    });
}

async fn apply_dispatch(state: &AppState, action: DispatchAction) -> Result<(), String> {
    match action {
        DispatchAction::Noop => Ok(()),
        DispatchAction::Reject(msg) => Err(msg),
        DispatchAction::EnqueueTrainingJob {
            job_id,
            tenant_id,
            project_id,
            priority,
        } => {
            if let Some(base) = &state.scheduler_url {
                let url = format!("{}/v1/queue", base.trim_end_matches('/'));
                let body = serde_json::json!({
                    "job_id": job_id,
                    "tenant_id": tenant_id,
                    "project_id": project_id,
                    "priority": priority,
                });
                let resp =
                    state.http.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    return Err(format!("scheduler enqueue status {}", resp.status()));
                }
                Ok(())
            } else {
                // Local-only path: admit immediately so single-process demos work.
                let tenant = TenantId::try_from(tenant_id.as_str()).map_err(|e| e.to_string())?;
                let job = TrainingJobId::try_from(job_id.as_str()).map_err(|e| e.to_string())?;
                {
                    let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
                    reg.admit_job(tenant, job).map_err(|e| e.to_string())?;
                }
                // Chain admitted event for allocate step.
                emit_event(
                    state,
                    tenant,
                    "training_job",
                    &job_id,
                    "training_job.admitted",
                    serde_json::json!({
                        "job_id": job_id,
                        "tenant_id": tenant_id,
                        "cpu_cores": 2,
                        "memory_mib": 4096
                    }),
                )
                .await;
                Ok(())
            }
        }
        DispatchAction::AdmitTrainingJob { job_id, tenant_id } => {
            let tenant = TenantId::try_from(tenant_id.as_str()).map_err(|e| e.to_string())?;
            let job = TrainingJobId::try_from(job_id.as_str()).map_err(|e| e.to_string())?;
            let mut reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
            // Idempotent: if already admitted, treat as success.
            match reg.admit_job(tenant, job) {
                Ok(_) => Ok(()),
                Err(e) if e.kind == moqentra_types::ErrorKind::Conflict => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
        DispatchAction::AllocateNode {
            attempt_id,
            cpu_cores,
            memory_mib,
        } => {
            if let Some(base) = &state.node_agent_url {
                let url = format!("{}/v1/allocations", base.trim_end_matches('/'));
                // Node-agent requires a UUID attempt id.
                let body = serde_json::json!({
                    "attempt_id": attempt_id,
                    "cpu_cores": cpu_cores,
                    "memory_mib": memory_mib,
                    "device_count": 0,
                    "fencing_token": 1
                });
                let resp =
                    state.http.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    return Err(format!("node-agent allocate status {}", resp.status()));
                }
                Ok(())
            } else {
                // No node-agent configured — succeed without remote allocation.
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectStorageHealthCheck;
    use super::*;
    use crate::import::InMemoryImportJobStore;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use moqentra_auth::{HmacValidator, InMemoryAuditLog, ServiceAccountValidator};
    use moqentra_domain::application::{ApplicationNode, Port};
    use moqentra_object_store::{InMemoryObjectStore, InMemoryUploadSessionStore};
    use moqentra_observability::LivenessCheck;
    use moqentra_types::UserId;
    use std::collections::BTreeMap;
    use tower::ServiceExt;

    fn empty_regs(
        tokens: CompositeTokenValidator,
        require_auth: bool,
        authorizer: Authorizer,
    ) -> AppState {
        let object_store: Arc<dyn ObjectStorage + Send + Sync> =
            Arc::new(InMemoryObjectStore::new());
        let metrics = Arc::new(MetricsRegistry::default());
        let mut health = CompositeHealthCheck::new();
        health.add(Arc::new(LivenessCheck));
        health.add(Arc::new(ObjectStorageHealthCheck {
            store: object_store.clone(),
        }));
        let health = Arc::new(health);
        AppState {
            compiler: ApplicationCompiler::new(),
            tokens,
            require_auth,
            authorizer: Arc::new(Mutex::new(authorizer)),
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
            audit: Arc::new(InMemoryAuditLog::new()),
            object_store,
            upload_sessions: Arc::new(InMemoryUploadSessionStore::new()),
            upload_sig_secret: "test-upload-secret".to_string(),
            import_jobs: Arc::new(InMemoryImportJobStore::new()),
            media_validator: Arc::new(moqentra_object_store::DefaultMediaValidator),
            db_pool: None,
            scheduler_url: None,
            node_agent_url: None,
            http: reqwest::Client::new(),
        }
    }

    fn open_state() -> AppState {
        empty_regs(
            CompositeTokenValidator::new(None, None),
            false,
            Authorizer::new(),
        )
    }

    fn authed_state() -> (AppState, String, TenantId, ProjectId, UserId) {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let user = UserId::new_v7(&gen);
        let mut authz = Authorizer::new();
        authz.assign_role(user, tenant, Role::MlEngineer);
        authz.add_project_member(user, tenant, project);

        let mut creds = HashMap::new();
        creds.insert("svc-token-1".to_string(), "worker".to_string());
        let hmac = HmacValidator::new("test-secret", "https://moqentra.test", "moqentra");
        let tokens =
            CompositeTokenValidator::new(Some(hmac), Some(ServiceAccountValidator::new(creds)));

        let state = empty_regs(tokens, true, authz);
        (state, "test-secret".to_string(), tenant, project, user)
    }

    fn mint_token(user: UserId, secret: &str) -> String {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        use moqentra_auth::TokenClaims;
        let claims = TokenClaims {
            sub: user.to_string(),
            iss: "https://moqentra.test".to_string(),
            aud: "moqentra".to_string(),
            exp: usize::MAX,
            iat: 0,
            roles: vec!["ml_engineer".to_string()],
            tenant_ids: vec![],
            nonce: None,
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn healthz_ok() {
        let app = app_router(open_state());
        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn compile_rejects_cycle() {
        let app = app_router(open_state());
        let mut nodes = BTreeMap::new();
        let port = |n: &str| Port {
            name: n.to_string(),
            port_type: "image".to_string(),
            schema: "any".to_string(),
        };
        let node = |id: &str| ApplicationNode {
            id: id.to_string(),
            node_type: "infer".to_string(),
            version: "1".to_string(),
            deprecated: false,
            inputs: vec![port("in")],
            outputs: vec![port("out")],
            parameters: BTreeMap::new(),
            resource_request: "small".to_string(),
            capabilities: vec![],
            bindings: BTreeMap::new(),
        };
        nodes.insert("a".to_string(), node("a"));
        nodes.insert("b".to_string(), node("b"));
        let tenant = TenantId::new_v7(&RandomIdGenerator);
        let body = serde_json::json!({
            "spec": {
                "nodes": nodes,
                "edges": [
                    ["a", "out", "b", "in"],
                    ["b", "out", "a", "in"]
                ]
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/applications:compile")
                    .header("content-type", "application/json")
                    .header("x-tenant-id", tenant.to_string())
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn dataset_create_requires_auth_when_configured() {
        let (state, _secret, tenant, project, _user) = authed_state();
        let app = app_router(state);
        let body = serde_json::json!({
            "name": "ds1",
            "project_id": project.to_string()
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/datasets")
                    .header("content-type", "application/json")
                    .header("x-tenant-id", tenant.to_string())
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn dataset_create_and_get_with_jwt() {
        let (state, secret, tenant, project, user) = authed_state();
        let token = mint_token(user, &secret);
        let app = app_router(state);

        let body = serde_json::json!({
            "name": "ds1",
            "project_id": project.to_string()
        });
        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/datasets")
                    .header("content-type", "application/json")
                    .header("x-tenant-id", tenant.to_string())
                    .header("x-project-id", project.to_string())
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(create.into_body(), 1024 * 1024).await.unwrap();
        let created: DatasetResponse = serde_json::from_slice(&bytes).unwrap();

        let get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/datasets/{}", created.id))
                    .header("x-tenant-id", tenant.to_string())
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cross_tenant_dataset_hidden() {
        let (state, secret, tenant, project, user) = authed_state();
        let token = mint_token(user, &secret);
        let app = app_router(state);
        let body = serde_json::json!({
            "name": "secret-ds",
            "project_id": project.to_string()
        });
        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/datasets")
                    .header("content-type", "application/json")
                    .header("x-tenant-id", tenant.to_string())
                    .header("x-project-id", project.to_string())
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(create.into_body(), 1024 * 1024).await.unwrap();
        let created: DatasetResponse = serde_json::from_slice(&bytes).unwrap();

        let other = TenantId::new_v7(&RandomIdGenerator);
        let get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/datasets/{}", created.id))
                    .header("x-tenant-id", other.to_string())
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Cross-tenant read is not-found (no existence leak) or forbidden.
        assert!(
            get.status() == StatusCode::NOT_FOUND || get.status() == StatusCode::FORBIDDEN,
            "status={}",
            get.status()
        );
    }

    #[tokio::test]
    async fn local_outbox_dispatch_admits_queued_job() {
        use moqentra_domain::training::{
            DistributedConfig, ParameterSchema, ResourceRequest, TrainingJobSpec, TrainingJobState,
        };
        use moqentra_types::{DatasetVersionId, ExperimentId};

        let state = open_state();
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let ds_ver = DatasetVersionId::new_v7(&gen);
        let exp = Experiment::new(
            ExperimentId::new_v7(&gen),
            tenant,
            project,
            "e1",
            ds_ver,
            "loss",
        )
        .unwrap();
        let exp_id = exp.id;
        {
            let mut reg = state.training.lock().unwrap();
            reg.create_experiment(exp).unwrap();
        }
        let job =
            TrainingJob::new(
                TrainingJobId::new_v7(&gen),
                exp_id,
                tenant,
                project,
                TrainingJobSpec {
                    code_digest:
                        "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                            .into(),
                    image_digest:
                        "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d"
                            .into(),
                    dataset_version_id: ds_ver,
                    hyperparameters: ParameterSchema {
                        argv: vec!["train".into()],
                        env: Default::default(),
                        config_files: Default::default(),
                    },
                    seed: 1,
                    resources: ResourceRequest {
                        replicas: 1,
                        cpu_milli: 1000,
                        memory_mib: 2048,
                        ephemeral_storage_mib: 1024,
                        accelerator_kind: None,
                        accelerator_count: 0,
                        topology: None,
                    },
                    distributed: DistributedConfig::Single,
                    max_attempts: 1,
                    deadline_seconds: 60,
                },
            )
            .unwrap();
        let job_id = job.id;
        {
            let mut reg = state.training.lock().unwrap();
            reg.create_job(job).unwrap();
        }

        // Local dispatch (no scheduler_url) admits immediately.
        apply_dispatch(
            &state,
            DispatchAction::EnqueueTrainingJob {
                job_id: job_id.to_string(),
                tenant_id: tenant.to_string(),
                project_id: project.to_string(),
                priority: 1,
            },
        )
        .await
        .unwrap();

        let reg = state.training.lock().unwrap();
        let admitted = reg.get_job(tenant, job_id).unwrap();
        assert!(matches!(admitted.state, TrainingJobState::Admitted));
    }

    #[tokio::test]
    async fn upload_session_create_list_upload_complete() {
        let (state, secret, tenant, project, user) = authed_state();
        let app = app_router(state);
        let token = mint_token(user, &secret);

        // Create an empty dataset so the version and upload target reference a dataset.
        let create_dataset = Request::builder()
            .method("POST")
            .uri("/v1/datasets")
            .header("content-type", "application/json")
            .header("x-tenant-id", tenant.to_string())
            .header("x-project-id", project.to_string())
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(format!(
                "{{\"name\":\"ds\",\"project_id\":\"{}\"}}",
                project
            )))
            .unwrap();
        let resp = app.clone().oneshot(create_dataset).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let version_id = "ver-1";
        let create_session = Request::builder()
            .method("POST")
            .uri("/v1/upload-sessions")
            .header("content-type", "application/json")
            .header("x-tenant-id", tenant.to_string())
            .header("x-project-id", project.to_string())
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(format!(
                "{{\"resource_type\":\"datasets\",\"resource_id\":\"ds\",\"version_id\":\"{version_id}\",\"name\":\"data.bin\",\"media_type\":\"application/octet-stream\",\"part_size\":5,\"total_size\":12}}"
            )))
            .unwrap();
        let resp = app.clone().oneshot(create_session).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let session: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let session_id = session["id"].as_str().unwrap().to_string();
        assert_eq!(session["total_size"].as_u64().unwrap(), 12);

        // Upload 3 parts.
        for n in 1..=3 {
            let payload = match n {
                1 => "12345",
                2 => "67890",
                _ => "ab",
            };
            let upload = Request::builder()
                .method("POST")
                .uri(format!("/v1/upload-sessions/{session_id}/parts/{n}"))
                .header("content-type", "application/octet-stream")
                .header("x-tenant-id", tenant.to_string())
                .header("x-project-id", project.to_string())
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(payload))
                .unwrap();
            let resp = app.clone().oneshot(upload).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        }

        // List part upload URLs; all parts are complete, so result is empty.
        let list_urls = Request::builder()
            .method("GET")
            .uri(format!("/v1/upload-sessions/{session_id}/part-urls"))
            .header("x-tenant-id", tenant.to_string())
            .header("x-project-id", project.to_string())
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(list_urls).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let complete = Request::builder()
            .method("POST")
            .uri(format!("/v1/upload-sessions/{session_id}/complete"))
            .header("x-tenant-id", tenant.to_string())
            .header("x-project-id", project.to_string())
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(complete).await.unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        if status != StatusCode::OK {
            eprintln!("complete failed: {:?}", String::from_utf8_lossy(&body));
        }
        assert_eq!(status, StatusCode::OK);
        let session: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(session["state"], "Completed");
    }
}
