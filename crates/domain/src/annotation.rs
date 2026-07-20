//! Annotation project, task, lease and conflict resolution domain.

use moqentra_types::{
    AnnotationId, AnnotationProjectId, AnnotationTaskId, AssetId, ProjectId, TenantId, UserId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// Supported annotation task types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    ImageClassification,
    VideoClassification,
    BoundingBox,
    Polygon,
    KeyPoint,
    ObjectTracking,
    Relation,
}

/// A label entry in the ontology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
    pub parent_id: Option<String>,
    pub metadata: Value,
}

/// Annotation project ontology and tool configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ontology {
    pub schema_version: String,
    pub labels: Vec<Label>,
    pub tools: Vec<String>,
    pub attributes: Value,
}

impl Ontology {
    pub fn new(labels: Vec<Label>, tools: Vec<String>) -> Self {
        Self {
            schema_version: "v1".to_string(),
            labels,
            tools,
            attributes: Value::Null,
        }
    }
}

/// State of an annotation project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationProjectState {
    Draft,
    Active,
    Archived,
}

/// An annotation project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationProject {
    pub id: AnnotationProjectId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub dataset_version_id: moqentra_types::DatasetVersionId,
    pub name: String,
    pub task_type: TaskType,
    pub ontology: Ontology,
    pub state: AnnotationProjectState,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl AnnotationProject {
    pub fn new(
        id: AnnotationProjectId,
        tenant_id: TenantId,
        project_id: ProjectId,
        dataset_version_id: moqentra_types::DatasetVersionId,
        name: impl Into<String>,
        task_type: TaskType,
        ontology: Ontology,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            tenant_id,
            project_id,
            dataset_version_id,
            name: name.into(),
            task_type,
            ontology,
            state: AnnotationProjectState::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn activate(&mut self) {
        self.state = AnnotationProjectState::Active;
        self.updated_at = UtcTimestamp::now();
    }

    pub fn archive(&mut self) {
        self.state = AnnotationProjectState::Archived;
        self.updated_at = UtcTimestamp::now();
    }
}

/// State of an annotation task assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationTaskState {
    Pending,
    Assigned,
    InProgress,
    Submitted,
    Reviewing,
    Returned,
    Approved,
}

/// A task lease with fencing token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLease {
    pub annotation_task_id: AnnotationTaskId,
    pub assignee_id: UserId,
    pub fencing_token: u64,
    pub expires_at: UtcTimestamp,
}

impl TaskLease {
    pub fn is_valid(&self, clock: UtcTimestamp) -> bool {
        clock.is_before(self.expires_at)
    }

    pub fn renew(
        &mut self,
        clock: UtcTimestamp,
        ttl_seconds: i64,
    ) -> Result<(), moqentra_types::Error> {
        if !self.is_valid(clock) {
            return Err(moqentra_types::Error::conflict("lease expired"));
        }
        if ttl_seconds <= 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "ttl must be positive",
            ));
        }
        let duration = std::time::Duration::from_secs(ttl_seconds as u64);
        self.expires_at = clock
            .add_std_duration(duration)
            .ok_or_else(|| moqentra_types::Error::internal("overflow"))?;
        self.fencing_token += 1;
        Ok(())
    }
}

/// An annotation task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationTask {
    pub id: AnnotationTaskId,
    pub project_id: AnnotationProjectId,
    pub tenant_id: TenantId,
    pub state: AnnotationTaskState,
    pub assignee: Option<UserId>,
    pub asset_ids: Vec<AssetId>,
    pub lease: Option<TaskLease>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl AnnotationTask {
    pub fn new(
        id: AnnotationTaskId,
        project_id: AnnotationProjectId,
        tenant_id: TenantId,
        asset_ids: Vec<AssetId>,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            project_id,
            tenant_id,
            state: AnnotationTaskState::Pending,
            assignee: None,
            asset_ids,
            lease: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn assign(
        &mut self,
        assignee: UserId,
        fencing_token: u64,
        expires_at: UtcTimestamp,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.state,
            AnnotationTaskState::Pending
                | AnnotationTaskState::Assigned
                | AnnotationTaskState::Returned
        ) {
            return Err(moqentra_types::Error::conflict("task cannot be assigned"));
        }
        self.assignee = Some(assignee);
        self.lease = Some(TaskLease {
            annotation_task_id: self.id,
            assignee_id: assignee,
            fencing_token,
            expires_at,
        });
        self.state = AnnotationTaskState::Assigned;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn start(
        &mut self,
        assignee: UserId,
        fencing_token: u64,
    ) -> Result<(), moqentra_types::Error> {
        self.check_lease(assignee, fencing_token)?;
        self.state = AnnotationTaskState::InProgress;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn submit(
        &mut self,
        assignee: UserId,
        fencing_token: u64,
    ) -> Result<(), moqentra_types::Error> {
        self.check_lease(assignee, fencing_token)?;
        if !matches!(
            self.state,
            AnnotationTaskState::InProgress | AnnotationTaskState::Returned
        ) {
            return Err(moqentra_types::Error::conflict("task cannot be submitted"));
        }
        self.state = AnnotationTaskState::Submitted;
        self.lease = None;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn return_for_rework(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.state,
            AnnotationTaskState::Submitted | AnnotationTaskState::Reviewing
        ) {
            return Err(moqentra_types::Error::conflict("task cannot be returned"));
        }
        self.state = AnnotationTaskState::Returned;
        self.assignee = None;
        self.lease = None;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn approve(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.state,
            AnnotationTaskState::Submitted | AnnotationTaskState::Reviewing
        ) {
            return Err(moqentra_types::Error::conflict("task cannot be approved"));
        }
        self.state = AnnotationTaskState::Approved;
        self.lease = None;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn check_lease(
        &self,
        assignee: UserId,
        fencing_token: u64,
    ) -> Result<(), moqentra_types::Error> {
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| moqentra_types::Error::conflict("no active lease"))?;
        if lease.assignee_id != assignee {
            return Err(moqentra_types::Error::permission_denied(
                "task assigned to another user",
            ));
        }
        if lease.fencing_token != fencing_token {
            return Err(moqentra_types::Error::conflict("stale fencing token"));
        }
        if !lease.is_valid(UtcTimestamp::now()) {
            return Err(moqentra_types::Error::conflict("lease expired"));
        }
        Ok(())
    }
}

/// A single annotation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: AnnotationId,
    pub task_id: AnnotationTaskId,
    pub asset_id: AssetId,
    pub revision: u64,
    pub client_update_id: String,
    pub actor_id: UserId,
    pub payload: Value,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

/// Result of an autosave attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutosaveResult {
    Accepted(Annotation),
    Conflict { server_revision: u64, diff: Value },
}

/// Annotation revision log for an asset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnnotationLog {
    annotations: HashMap<AnnotationId, Annotation>,
    revisions: BTreeMap<u64, Vec<AnnotationId>>,
    next_revision: u64,
}

impl AnnotationLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn latest_revision(&self) -> u64 {
        self.next_revision.saturating_sub(1)
    }

    pub fn autosave(&mut self, mut annotation: Annotation) -> AutosaveResult {
        let revision = self.next_revision;
        if let Some(existing) = self.annotations.get(&annotation.id) {
            if existing.client_update_id == annotation.client_update_id {
                // Idempotent duplicate.
                return AutosaveResult::Accepted(existing.clone());
            }
            if existing.revision >= annotation.revision {
                return AutosaveResult::Conflict {
                    server_revision: existing.revision,
                    diff: serde_json::json!({"server": existing.payload, "client": annotation.payload}),
                };
            }
        }
        annotation.revision = revision;
        self.next_revision += 1;
        self.annotations.insert(annotation.id, annotation.clone());
        self.revisions.entry(revision).or_default().push(annotation.id);
        self.revisions.retain(|r, _| *r >= revision.saturating_sub(10));
        AutosaveResult::Accepted(annotation)
    }

    pub fn get(&self, id: AnnotationId) -> Option<&Annotation> {
        self.annotations.get(&id)
    }
}

/// Export format support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Coco,
    Voc,
    Yolo,
    Native,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{
        AnnotationId, AnnotationProjectId, AnnotationTaskId, AssetId, RandomIdGenerator, TenantId,
        UserId,
    };

    #[test]
    fn task_assignment_and_lease() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let _project = ProjectId::new_v7(&gen);
        let mut task = AnnotationTask::new(
            AnnotationTaskId::new_v7(&gen),
            AnnotationProjectId::new_v7(&gen),
            tenant,
            vec![AssetId::new_v7(&gen)],
        );
        let user = UserId::new_v7(&gen);
        let now = UtcTimestamp::now();
        let ttl = std::time::Duration::from_secs(60);
        let expires = now.add_std_duration(ttl).unwrap();
        task.assign(user, 1, expires).unwrap();
        task.start(user, 1).unwrap();
        assert!(matches!(task.state, AnnotationTaskState::InProgress));
    }

    #[test]
    fn stale_fencing_token_rejected() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let mut task = AnnotationTask::new(
            AnnotationTaskId::new_v7(&gen),
            AnnotationProjectId::new_v7(&gen),
            tenant,
            vec![AssetId::new_v7(&gen)],
        );
        let user = UserId::new_v7(&gen);
        let now = UtcTimestamp::now();
        let expires = now.add_std_duration(std::time::Duration::from_secs(60)).unwrap();
        task.assign(user, 1, expires).unwrap();
        assert!(task.start(user, 0).is_err());
    }

    #[test]
    fn autosave_idempotent_and_conflict() {
        let gen = RandomIdGenerator;
        let mut log = AnnotationLog::new();
        let annotation = Annotation {
            id: AnnotationId::new_v7(&gen),
            task_id: AnnotationTaskId::new_v7(&gen),
            asset_id: AssetId::new_v7(&gen),
            revision: 0,
            client_update_id: "update-1".to_string(),
            actor_id: UserId::new_v7(&gen),
            payload: serde_json::json!({"box": [1, 2, 3, 4]}),
            created_at: UtcTimestamp::now(),
            updated_at: UtcTimestamp::now(),
        };
        let AutosaveResult::Accepted(first) = log.autosave(annotation.clone()) else {
            panic!("expected accepted");
        };
        assert_eq!(first.revision, 0);

        let AutosaveResult::Accepted(second) = log.autosave(annotation.clone()) else {
            panic!("expected idempotent accepted");
        };
        assert_eq!(second.revision, 0);

        let mut outdated = annotation.clone();
        outdated.revision = 0;
        outdated.client_update_id = "update-2".to_string();
        assert!(matches!(
            log.autosave(outdated),
            AutosaveResult::Conflict { .. }
        ));
    }
}
