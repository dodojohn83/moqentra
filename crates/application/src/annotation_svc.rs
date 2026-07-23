//! Annotation project application services.

use crate::coco::CocoDataset;
use moqentra_domain::annotation::{
    Annotation, AnnotationLog, AnnotationProject, AnnotationProjectState, AnnotationTask,
    AnnotationTaskState, AutosaveResult, TaskType,
};
use moqentra_domain::dataset::AssetRef;
use moqentra_types::{
    AnnotationProjectId, AnnotationTaskId, AssetId, Error, ProjectId, TenantId, UserId,
};
use std::collections::BTreeMap;

/// In-memory annotation project registry.
#[derive(Debug, Default)]
pub struct InMemoryAnnotationRegistry {
    projects: BTreeMap<AnnotationProjectId, AnnotationProject>,
    tasks: BTreeMap<AnnotationTaskId, AnnotationTask>,
    logs: BTreeMap<AnnotationTaskId, AnnotationLog>,
}

impl InMemoryAnnotationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state from durable storage (restart recovery).
    pub fn hydrate(&mut self, projects: Vec<AnnotationProject>, tasks: Vec<AnnotationTask>) {
        self.projects.clear();
        self.tasks.clear();
        // Logs are derived from runtime activity; not recovered from PG yet.
        self.logs.clear();
        for p in projects {
            self.projects.insert(p.id, p);
        }
        for t in tasks {
            self.tasks.insert(t.id, t);
        }
    }

    /// Create an annotation project.
    pub fn create_project(
        &mut self,
        project: AnnotationProject,
    ) -> Result<AnnotationProject, Error> {
        if self.projects.contains_key(&project.id) {
            return Err(Error::conflict("annotation project already exists"));
        }
        self.projects.insert(project.id, project.clone());
        Ok(project)
    }

    /// Get project with tenant isolation.
    pub fn get_project(
        &self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self.projects.get(&id).ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        Ok(p)
    }

    /// List projects for a tenant.
    pub fn list_projects(
        &self,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Vec<&AnnotationProject> {
        self.projects
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .filter(|p| project_id.is_none_or(|pid| p.project_id == pid))
            .collect()
    }

    /// Activate a draft/archived annotation project.
    pub fn activate(
        &mut self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self
            .projects
            .get_mut(&id)
            .ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        p.activate()?;
        Ok(p)
    }

    /// Archive an active annotation project.
    pub fn archive(
        &mut self,
        tenant_id: TenantId,
        id: AnnotationProjectId,
    ) -> Result<&AnnotationProject, Error> {
        let p = self
            .projects
            .get_mut(&id)
            .ok_or_else(|| Error::not_found("annotation project"))?;
        if p.tenant_id != tenant_id {
            return Err(Error::not_found("annotation project"));
        }
        p.archive()?;
        Ok(p)
    }

    /// Create annotation tasks for a project, one per asset in `asset_ids`.
    pub fn create_tasks(
        &mut self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        task_ids: Vec<AnnotationTaskId>,
        asset_ids: Vec<AssetId>,
    ) -> Result<Vec<AnnotationTask>, Error> {
        let p = self.get_project(tenant_id, project_id)?;
        if !matches!(p.state, AnnotationProjectState::Active) {
            return Err(Error::conflict("project is not active"));
        }
        if task_ids.len() != asset_ids.len() {
            return Err(Error::invalid_argument(
                "task_ids and asset_ids must have the same length",
            ));
        }
        let mut created = Vec::with_capacity(task_ids.len());
        for (task_id, asset_id) in task_ids.into_iter().zip(asset_ids.into_iter()) {
            if self.tasks.contains_key(&task_id) {
                return Err(Error::conflict("task id already exists"));
            }
            let task = AnnotationTask::new(task_id, project_id, tenant_id, vec![asset_id])?;
            self.tasks.insert(task.id, task.clone());
            self.logs.entry(task.id).or_default();
            created.push(task);
        }
        Ok(created)
    }

    /// List tasks for a project, optionally filtered by state.
    pub fn list_tasks(
        &self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        state: Option<AnnotationTaskState>,
    ) -> Result<Vec<AnnotationTask>, Error> {
        self.get_project(tenant_id, project_id)?;
        Ok(self
            .tasks
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.project_id == project_id)
            .filter(|t| state.is_none_or(|s| t.state == s))
            .cloned()
            .collect())
    }

    /// Get a single task with tenant isolation.
    pub fn get_task(
        &self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
    ) -> Result<&AnnotationTask, Error> {
        let t = self.tasks.get(&task_id).ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        Ok(t)
    }

    /// Assign/claim a task with a time-bounded lease.
    pub fn assign_task(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
        assignee: UserId,
        fencing_token: u64,
        expires_at: moqentra_types::UtcTimestamp,
    ) -> Result<&AnnotationTask, Error> {
        let t = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        t.assign(assignee, fencing_token, expires_at)?;
        Ok(t)
    }

    /// Start a task (worker confirms they are actively annotating).
    pub fn start_task(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
        assignee: UserId,
        fencing_token: u64,
    ) -> Result<&AnnotationTask, Error> {
        let t = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        t.start(assignee, fencing_token)?;
        Ok(t)
    }

    /// Submit annotations for review.
    pub fn submit_task(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
        assignee: UserId,
        fencing_token: u64,
    ) -> Result<&AnnotationTask, Error> {
        let t = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        t.submit(assignee, fencing_token)?;
        Ok(t)
    }

    /// Return a submitted/reviewing task for rework.
    pub fn return_task(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
    ) -> Result<&AnnotationTask, Error> {
        let t = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        t.return_for_rework()?;
        Ok(t)
    }

    /// Approve a submitted/reviewing task.
    pub fn approve_task(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
    ) -> Result<&AnnotationTask, Error> {
        let t = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        t.approve()?;
        Ok(t)
    }

    /// Autosave an annotation for a task, with idempotent client_update_id.
    pub fn autosave(
        &mut self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
        annotation: Annotation,
    ) -> Result<AutosaveResult, Error> {
        let t = self.tasks.get(&task_id).ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        if !t.asset_ids.contains(&annotation.asset_id) {
            return Err(Error::invalid_argument(
                "asset does not belong to this task",
            ));
        }
        if t.lease.as_ref().is_none_or(|l| l.assignee_id != annotation.actor_id) {
            return Err(Error::permission_denied("task not assigned to actor"));
        }
        let log = self.logs.entry(task_id).or_default();
        log.autosave(annotation)
    }

    /// List annotations for a task.
    pub fn list_annotations(
        &self,
        tenant_id: TenantId,
        task_id: AnnotationTaskId,
    ) -> Result<Vec<Annotation>, Error> {
        let t = self.tasks.get(&task_id).ok_or_else(|| Error::not_found("annotation task"))?;
        if t.tenant_id != tenant_id {
            return Err(Error::not_found("annotation task"));
        }
        let log = self.logs.get(&task_id).ok_or_else(|| Error::not_found("annotation log"))?;
        Ok(log.get_all().into_iter().cloned().collect())
    }

    /// Export this project's annotations to COCO format.
    pub fn export_coco(
        &self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        task_type: TaskType,
        assets: &[AssetRef],
    ) -> Result<CocoDataset, Error> {
        self.get_project(tenant_id, project_id)?;
        let tasks: Vec<AnnotationTask> = self
            .tasks
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.project_id == project_id)
            .cloned()
            .collect();
        let mut annotations_by_task: BTreeMap<AnnotationTaskId, Vec<Annotation>> = BTreeMap::new();
        for (task_id, log) in &self.logs {
            if let Some(t) = self.tasks.get(task_id) {
                if t.tenant_id == tenant_id && t.project_id == project_id {
                    annotations_by_task
                        .insert(*task_id, log.get_all().into_iter().cloned().collect());
                }
            }
        }
        CocoDataset::export(project_id, task_type, &tasks, &annotations_by_task, assets)
    }

    /// Import COCO annotations into this project, creating approved tasks.
    pub fn import_coco(
        &mut self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        task_type: TaskType,
        assets: &[AssetRef],
        dataset: &CocoDataset,
    ) -> Result<Vec<AnnotationTaskId>, Error> {
        self.get_project(tenant_id, project_id)?;
        let pairs = dataset.import(project_id, tenant_id, task_type, assets)?;
        let mut ids = Vec::with_capacity(pairs.len());
        for (task, annotations) in pairs {
            if self.tasks.contains_key(&task.id) {
                return Err(Error::conflict("task id collision during import"));
            }
            let task_id = task.id;
            self.tasks.insert(task_id, task);
            let log = self.logs.entry(task_id).or_default();
            for annotation in annotations {
                log.autosave(annotation)?;
            }
            ids.push(task_id);
        }
        Ok(ids)
    }

    /// Export this project's annotations to LabelU native format.
    pub fn export_labelu(
        &self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        task_type: TaskType,
        assets: &[AssetRef],
    ) -> Result<crate::labelu::LabelUDataset, Error> {
        self.get_project(tenant_id, project_id)?;
        let tasks: Vec<AnnotationTask> = self
            .tasks
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.project_id == project_id)
            .cloned()
            .collect();
        let mut annotations_by_task: BTreeMap<AnnotationTaskId, Vec<Annotation>> = BTreeMap::new();
        for (task_id, log) in &self.logs {
            if let Some(t) = self.tasks.get(task_id) {
                if t.tenant_id == tenant_id && t.project_id == project_id {
                    annotations_by_task
                        .insert(*task_id, log.get_all().into_iter().cloned().collect());
                }
            }
        }
        crate::labelu::LabelUDataset::export(
            project_id,
            task_type,
            &tasks,
            &annotations_by_task,
            assets,
        )
    }

    /// Import LabelU native annotations into this project.
    pub fn import_labelu(
        &mut self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        task_type: TaskType,
        assets: &[AssetRef],
        dataset: &crate::labelu::LabelUDataset,
    ) -> Result<Vec<AnnotationTaskId>, Error> {
        self.get_project(tenant_id, project_id)?;
        let pairs = dataset.import(project_id, tenant_id, task_type, assets)?;
        let mut ids = Vec::with_capacity(pairs.len());
        for (task, annotations) in pairs {
            if self.tasks.contains_key(&task.id) {
                return Err(Error::conflict("task id collision during import"));
            }
            let task_id = task.id;
            self.tasks.insert(task_id, task);
            let log = self.logs.entry(task_id).or_default();
            for annotation in annotations {
                log.autosave(annotation)?;
            }
            ids.push(task_id);
        }
        Ok(ids)
    }

    /// Export this project's annotations to the platform intermediate format.
    pub fn export_platform(
        &self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
    ) -> Result<crate::platform::PlatformAnnotationDataset, Error> {
        self.get_project(tenant_id, project_id)?;
        let tasks: Vec<AnnotationTask> = self
            .tasks
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.project_id == project_id)
            .cloned()
            .collect();
        let mut annotations_by_task: BTreeMap<AnnotationTaskId, Vec<Annotation>> = BTreeMap::new();
        for (task_id, log) in &self.logs {
            if let Some(t) = self.tasks.get(task_id) {
                if t.tenant_id == tenant_id && t.project_id == project_id {
                    annotations_by_task
                        .insert(*task_id, log.get_all().into_iter().cloned().collect());
                }
            }
        }
        Ok(crate::platform::PlatformAnnotationDataset::export(
            project_id,
            &tasks,
            &annotations_by_task,
        ))
    }

    /// Import platform intermediate annotations into this project.
    pub fn import_platform(
        &mut self,
        tenant_id: TenantId,
        project_id: AnnotationProjectId,
        dataset: &crate::platform::PlatformAnnotationDataset,
    ) -> Result<Vec<AnnotationTaskId>, Error> {
        self.get_project(tenant_id, project_id)?;
        let pairs = dataset.import(project_id, tenant_id)?;
        let mut ids = Vec::with_capacity(pairs.len());
        for (task, annotations) in pairs {
            if self.tasks.contains_key(&task.id) {
                return Err(Error::conflict("task id collision during import"));
            }
            let task_id = task.id;
            self.tasks.insert(task_id, task);
            let log = self.logs.entry(task_id).or_default();
            for annotation in annotations {
                log.autosave(annotation)?;
            }
            ids.push(task_id);
        }
        Ok(ids)
    }

    /// Active project count (ops metric).
    pub fn active_count(&self, tenant_id: TenantId) -> usize {
        self.projects
            .values()
            .filter(|p| {
                p.tenant_id == tenant_id && matches!(p.state, AnnotationProjectState::Active)
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::annotation::{Ontology, TaskType};
    use moqentra_types::{DatasetVersionId, RandomIdGenerator};

    #[test]
    fn activate_annotation_project() {
        let gen = RandomIdGenerator;
        let mut reg = InMemoryAnnotationRegistry::new();
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let ap = AnnotationProject::new(
            AnnotationProjectId::new_v7(&gen),
            tenant,
            project,
            DatasetVersionId::new_v7(&gen),
            "labels",
            TaskType::BoundingBox,
            Ontology::new(vec![], vec!["rectTool".to_string()]),
        )
        .unwrap();
        let id = ap.id;
        reg.create_project(ap).unwrap();
        reg.activate(tenant, id).unwrap();
        assert_eq!(reg.active_count(tenant), 1);
    }
}
