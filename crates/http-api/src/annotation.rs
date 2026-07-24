//! Annotation task and review endpoints.

use crate::control_plane::{
    audit_write, authorize, check_rate_limit, persist_annotation_task, resolve_context, ApiError,
    AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use moqentra_application::coco::CocoDataset;
use moqentra_application::labelu::LabelUDataset;
use moqentra_application::platform::PlatformAnnotationDataset;
use moqentra_auth::{Action, AuditCategory, AuditOutcome, Resource};
use moqentra_domain::annotation::{Annotation, AnnotationTask};
use moqentra_types::{
    AnnotationId, AnnotationProjectId, AnnotationTaskId, AssetId, Principal, RandomIdGenerator,
    RequestContext, UserId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize)]
pub struct CreateTasksRequest {
    asset_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct AssignRequest {
    ttl_seconds: u64,
}

#[derive(Deserialize)]
pub struct AutosaveRequest {
    client_update_id: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
pub struct TaskResponse {
    id: String,
    project_id: String,
    state: String,
    assignee: Option<String>,
    asset_ids: Vec<String>,
    lease_expires_at: Option<String>,
}

#[derive(Serialize)]
pub struct AnnotationResponse {
    id: String,
    task_id: String,
    asset_id: String,
    revision: u64,
    client_update_id: String,
    actor_id: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
pub struct MediaUrlResponse {
    url: String,
    expires_at: String,
}

impl From<&AnnotationTask> for TaskResponse {
    fn from(t: &AnnotationTask) -> Self {
        Self {
            id: t.id.to_string(),
            project_id: t.project_id.to_string(),
            state: format!("{:?}", t.state),
            assignee: t.assignee.map(|u| u.to_string()),
            asset_ids: t.asset_ids.iter().map(|a| a.to_string()).collect(),
            lease_expires_at: t.lease.as_ref().map(|l| l.expires_at.to_string()),
        }
    }
}

impl From<&Annotation> for AnnotationResponse {
    fn from(a: &Annotation) -> Self {
        Self {
            id: a.id.to_string(),
            task_id: a.task_id.to_string(),
            asset_id: a.asset_id.to_string(),
            revision: a.revision,
            client_update_id: a.client_update_id.clone(),
            actor_id: a.actor_id.to_string(),
            payload: a.payload.clone(),
        }
    }
}

fn require_user(ctx: &RequestContext) -> Result<UserId, moqentra_types::Error> {
    match ctx.principal {
        Principal::User { id } => Ok(id),
        Principal::Anonymous => Err(moqentra_types::Error::permission_denied(
            "signed media url requires an authenticated user",
        )),
        Principal::Service { .. } => Err(moqentra_types::Error::permission_denied(
            "annotation tasks must be performed by a user",
        )),
    }
}

pub(crate) async fn create_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<CreateTasksRequest>,
) -> Result<(StatusCode, Json<Vec<TaskResponse>>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Create, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let gen = RandomIdGenerator;
    let asset_ids: Vec<AssetId> = req
        .asset_ids
        .iter()
        .map(|s| AssetId::try_from(s.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    let task_ids: Vec<AnnotationTaskId> =
        (0..asset_ids.len()).map(|_| AnnotationTaskId::new_v7(&gen)).collect();

    let created = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.create_tasks(ctx.tenant_id, project_id, task_ids, asset_ids)?
    };
    for t in &created {
        persist_annotation_task(&state, t).await;
    }

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_task.create",
        Resource::AnnotationTask,
        Some(&project_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(created.iter().map(|t| t.into()).collect()),
    ))
}

pub(crate) async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let tasks = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.list_tasks(ctx.tenant_id, project_id, None)?
    };
    Ok(Json(tasks.iter().map(|t| t.into()).collect()))
}

pub(crate) async fn get_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let t = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_task(ctx.tenant_id, task_id)?.clone()
    };
    Ok(Json((&t).into()))
}

pub(crate) async fn assign_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(req): Json<AssignRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let user_id = require_user(&ctx)?;
    let ttl = Duration::from_secs(req.ttl_seconds.clamp(30, 3600));
    let now = UtcTimestamp::now();
    let expires_at = now
        .add_std_duration(ttl)
        .ok_or_else(|| moqentra_types::Error::invalid_argument("invalid lease ttl"))?;
    let fencing_token = u64::try_from(now.as_offset().unix_timestamp())
        .map_err(|_| moqentra_types::Error::invalid_argument("fencing timestamp overflow"))?;

    let t = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.assign_task(ctx.tenant_id, task_id, user_id, fencing_token, expires_at)?
            .clone()
    };
    persist_annotation_task(&state, &t).await;
    Ok(Json((&t).into()))
}

pub(crate) async fn start_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let user_id = require_user(&ctx)?;
    let t = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let task = reg.get_task(ctx.tenant_id, task_id)?.clone();
        let token = task.lease.as_ref().map(|l| l.fencing_token).unwrap_or(0);
        reg.start_task(ctx.tenant_id, task_id, user_id, token)?.clone()
    };
    persist_annotation_task(&state, &t).await;
    Ok(Json((&t).into()))
}

pub(crate) async fn submit_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let user_id = require_user(&ctx)?;
    let t = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let task = reg.get_task(ctx.tenant_id, task_id)?.clone();
        let token = task.lease.as_ref().map(|l| l.fencing_token).unwrap_or(0);
        reg.submit_task(ctx.tenant_id, task_id, user_id, token)?.clone()
    };
    persist_annotation_task(&state, &t).await;

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_task.submit",
        Resource::AnnotationTask,
        Some(&task_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok(Json((&t).into()))
}

pub(crate) async fn return_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let t = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.return_task(ctx.tenant_id, task_id)?.clone()
    };
    persist_annotation_task(&state, &t).await;
    Ok(Json((&t).into()))
}

pub(crate) async fn approve_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let t = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.approve_task(ctx.tenant_id, task_id)?.clone()
    };
    persist_annotation_task(&state, &t).await;

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_task.approve",
        Resource::AnnotationTask,
        Some(&task_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok(Json((&t).into()))
}

pub(crate) async fn autosave_annotation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(req): Json<AutosaveRequest>,
) -> Result<(StatusCode, Json<AnnotationResponse>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let user_id = require_user(&ctx)?;
    let gen = RandomIdGenerator;
    let annotation = Annotation {
        id: AnnotationId::new_v7(&gen),
        task_id,
        asset_id: state
            .annotations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_task(ctx.tenant_id, task_id)?
            .asset_ids
            .first()
            .copied()
            .ok_or_else(|| moqentra_types::Error::not_found("task asset"))?,
        revision: 0,
        client_update_id: req.client_update_id,
        actor_id: user_id,
        payload: req.payload,
        created_at: UtcTimestamp::now(),
        updated_at: UtcTimestamp::now(),
    };

    let result = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.autosave(ctx.tenant_id, task_id, annotation)?
    };

    match result {
        moqentra_domain::annotation::AutosaveResult::Accepted(a) => {
            Ok((StatusCode::OK, Json((&a).into())))
        }
        moqentra_domain::annotation::AutosaveResult::Conflict {
            server_revision, ..
        } => Err(moqentra_types::Error::conflict(format!(
            "annotation conflict at revision {server_revision}"
        ))
        .into()),
    }
}

pub(crate) async fn list_annotations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<AnnotationResponse>>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let _project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let task_id = AnnotationTaskId::try_from(task_id.as_str())?;
    let annotations = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.list_annotations(ctx.tenant_id, task_id)?
    };
    Ok(Json(annotations.iter().map(|a| a.into()).collect()))
}

pub(crate) async fn export_coco(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<CocoDataset>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let (dataset_version_id, task_type) = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let project = reg.get_project(ctx.tenant_id, project_id)?;
        (project.dataset_version_id, project.task_type)
    };

    let assets = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_version(ctx.tenant_id, dataset_version_id)?.assets.clone()
    };

    let coco = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.export_coco(ctx.tenant_id, project_id, task_type, &assets)?
    };

    Ok(Json(coco))
}

pub(crate) async fn import_coco(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(dataset): Json<CocoDataset>,
) -> Result<(StatusCode, Json<Vec<String>>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let (dataset_version_id, task_type) = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let project = reg.get_project(ctx.tenant_id, project_id)?;
        (project.dataset_version_id, project.task_type)
    };

    let assets = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_version(ctx.tenant_id, dataset_version_id)?.assets.clone()
    };

    let ids = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.import_coco(ctx.tenant_id, project_id, task_type, &assets, &dataset)?
    };

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_project.import_coco",
        Resource::AnnotationTask,
        Some(&project_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ids.iter().map(|id| id.to_string()).collect()),
    ))
}

pub(crate) async fn export_labelu(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<LabelUDataset>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let (dataset_version_id, task_type) = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let project = reg.get_project(ctx.tenant_id, project_id)?;
        (project.dataset_version_id, project.task_type)
    };

    let assets = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_version(ctx.tenant_id, dataset_version_id)?.assets.clone()
    };

    let dataset = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.export_labelu(ctx.tenant_id, project_id, task_type, &assets)?
    };

    Ok(Json(dataset))
}

pub(crate) async fn import_labelu(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(dataset): Json<LabelUDataset>,
) -> Result<(StatusCode, Json<Vec<String>>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let (dataset_version_id, task_type) = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        let project = reg.get_project(ctx.tenant_id, project_id)?;
        (project.dataset_version_id, project.task_type)
    };

    let assets = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_version(ctx.tenant_id, dataset_version_id)?.assets.clone()
    };

    let ids = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.import_labelu(ctx.tenant_id, project_id, task_type, &assets, &dataset)?
    };

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_project.import_labelu",
        Resource::AnnotationTask,
        Some(&project_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ids.iter().map(|id| id.to_string()).collect()),
    ))
}

pub(crate) async fn export_platform(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<PlatformAnnotationDataset>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let dataset = {
        let reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.export_platform(ctx.tenant_id, project_id)?
    };

    Ok(Json(dataset))
}

pub(crate) async fn import_platform(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(dataset): Json<PlatformAnnotationDataset>,
) -> Result<(StatusCode, Json<Vec<String>>), ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Update, Resource::AnnotationTask).await?;

    let project_id = AnnotationProjectId::try_from(project_id.as_str())?;
    let ids = {
        let mut reg = state.annotations.lock().unwrap_or_else(|e| e.into_inner());
        reg.import_platform(ctx.tenant_id, project_id, &dataset)?
    };

    audit_write(
        &state,
        &ctx,
        AuditCategory::Write,
        "annotation_project.import_platform",
        Resource::AnnotationTask,
        Some(&project_id.to_string()),
        AuditOutcome::Success,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ids.iter().map(|id| id.to_string()).collect()),
    ))
}

pub(crate) async fn get_asset_media_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(asset_id): Path<String>,
) -> Result<Json<MediaUrlResponse>, ApiError> {
    let ctx = resolve_context(&state, &headers).await?;
    check_rate_limit(&state, ctx.tenant_id)?;
    authorize(&state, &ctx, Action::Read, Resource::Dataset).await?;
    let _ = require_user(&ctx)?;

    let asset_id = AssetId::try_from(asset_id.as_str())?;
    let asset = {
        let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
        reg.find_asset(ctx.tenant_id, asset_id)
    }
    .ok_or_else(|| moqentra_types::Error::not_found("asset"))?;

    let ttl = Duration::from_secs(900);
    let url = state.object_store.presigned_get_url(&asset.1.object_key, ttl).await?;
    let expires_at = UtcTimestamp::now()
        .add_std_duration(ttl)
        .ok_or_else(|| moqentra_types::Error::internal("overflow"))?;
    Ok(Json(MediaUrlResponse {
        url,
        expires_at: expires_at.to_string(),
    }))
}
